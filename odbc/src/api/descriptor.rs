use crate::api::CDataType;
use crate::api::encoding::{OdbcEncoding, write_string_bytes_i32, write_string_chars};
use crate::api::error::AssociatedStatementNotPreparedSnafu;
use crate::api::handle_registry::HandleGuard;
use crate::api::types::{DescriptorAccess, DescriptorKind, State, Statement, StatementState};
use crate::api::utils::{
    IrdFieldValue, compute_ird_concise_type, compute_ird_field, compute_ird_name,
    compute_ird_nullable, compute_ird_octet_length, compute_ird_precision, compute_ird_scale,
    compute_ird_verbose_type,
};
use crate::api::{DescField, OdbcResult, desc_from_handle};
use crate::conversion::warning::Warnings;
use arrow::array::RecordBatchReader;
use odbc_sys as sql;
use tracing;

/// Get a descriptor field value.
pub fn get_desc_field<E: OdbcEncoding>(
    desc_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    tracing::debug!(
        "get_desc_field: desc_handle={:?}, rec_number={}, field_identifier={}",
        desc_handle,
        rec_number,
        field_identifier
    );

    if value_ptr.is_null() {
        tracing::error!("get_desc_field: value_ptr is null");
        return crate::api::error::NullPointerSnafu.fail();
    }

    if rec_number < 0 {
        tracing::error!("get_desc_field: invalid negative rec_number {}", rec_number);
        return crate::api::error::InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    if buffer_length < 0 {
        return crate::api::error::InvalidBufferLengthSnafu {
            length: buffer_length as i64,
        }
        .fail();
    }

    let field = DescField::try_from(field_identifier)?;
    let access = desc_from_handle(desc_handle)?;

    if field == DescField::AllocType {
        let alloc_type: sql::SmallInt = match &access {
            DescriptorAccess::Implicit { .. } => 1, // SQL_DESC_ALLOC_AUTO
            DescriptorAccess::Explicit { .. } => 2, // SQL_DESC_ALLOC_USER
        };
        unsafe {
            std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, alloc_type);
        }
        return Ok(());
    }

    match access {
        DescriptorAccess::Implicit { guard, kind } => {
            let inner = guard.inner.lock();
            if inner.state.as_ref().is_need_data() {
                return crate::api::error::InvalidDuringDaeSnafu.fail();
            }
            match kind {
                DescriptorKind::Ard => get_ard_field(&inner.ard, rec_number, field, value_ptr),
                DescriptorKind::Ird => get_ird_field::<E>(
                    &inner.ird,
                    rec_number,
                    field,
                    value_ptr,
                    buffer_length,
                    string_length_ptr,
                    &guard,
                    &inner.state,
                    warnings,
                ),
                DescriptorKind::Apd => get_apd_field(&inner.apd, rec_number, field, value_ptr),
                DescriptorKind::Ipd => get_ipd_field::<E>(
                    &inner.ipd,
                    rec_number,
                    field,
                    value_ptr,
                    buffer_length,
                    string_length_ptr,
                    warnings,
                ),
            }
        }
        DescriptorAccess::Explicit { desc } => {
            let desc = desc.lock();
            get_ard_field(&desc, rec_number, field, value_ptr)
        }
    }
}

fn get_ard_field(
    desc: &crate::api::ArdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
) -> OdbcResult<()> {
    if rec_number == 0 {
        match field {
            DescField::Count => {
                let count = desc.desc_count();
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        count as sql::SmallInt,
                    );
                }
                Ok(())
            }
            DescField::ArraySize => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::ULen,
                        desc.array_size as sql::ULen,
                    );
                }
                Ok(())
            }
            DescField::BindType => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::ULen,
                        desc.bind_type as sql::ULen,
                    );
                }
                Ok(())
            }
            DescField::BindOffsetPtr => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut *mut sql::Len,
                        desc.bind_offset_ptr,
                    );
                }
                Ok(())
            }
            _ => {
                tracing::warn!("get_desc_field: unsupported ARD header field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        }
    } else {
        if !is_valid_ard_record_field(field) {
            return crate::api::error::InvalidDescriptorFieldIdSnafu {
                field_id: field as i16,
            }
            .fail();
        }

        let column_number = rec_number as u16;
        let binding = match desc.bindings.get(&column_number) {
            Some(b) => b,
            None => {
                tracing::debug!(
                    "get_desc_field: no binding for record {}, returning SQL_NO_DATA",
                    rec_number
                );
                return crate::api::error::NoMoreDataSnafu.fail();
            }
        };

        match field {
            DescField::ConciseType => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        binding.target_type as sql::SmallInt,
                    );
                }
                Ok(())
            }
            DescField::Type => {
                let concise = binding.target_type as sql::SmallInt;
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        verbose_type_from_concise(concise),
                    );
                }
                Ok(())
            }
            DescField::DatetimeIntervalCode => {
                let concise = binding.target_type as sql::SmallInt;
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        datetime_interval_code(concise),
                    );
                }
                Ok(())
            }
            DescField::Precision => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        binding.precision.unwrap_or(0),
                    );
                }
                Ok(())
            }
            DescField::Scale => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        binding.scale.unwrap_or(0),
                    );
                }
                Ok(())
            }
            DescField::OctetLength => {
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut sql::Len, binding.buffer_length);
                }
                Ok(())
            }
            DescField::DataPtr => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::Pointer,
                        binding.target_value_ptr,
                    );
                }
                Ok(())
            }
            DescField::IndicatorPtr => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut *mut sql::Len,
                        binding.indicator_ptr,
                    );
                }
                Ok(())
            }
            DescField::OctetLengthPtr => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut *mut sql::Len,
                        binding.octet_length_ptr,
                    );
                }
                Ok(())
            }
            DescField::DatetimeIntervalPrecision => {
                let dip = binding.datetime_interval_precision.unwrap_or(2);
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, dip);
                }
                Ok(())
            }
            DescField::Length => {
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut sql::ULen, binding.length);
                }
                Ok(())
            }
            _ => {
                tracing::warn!("get_desc_field: unsupported ARD record field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn get_ird_field<E: OdbcEncoding>(
    desc: &crate::api::IrdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
    guard: &HandleGuard<Statement>,
    state: &State<StatementState>,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let schema = match state.as_ref() {
        StatementState::Prepared { schema } => schema.clone(),
        StatementState::QueryExecuted { reader, .. } => reader.schema(),
        StatementState::Fetching { record_batch, .. } => record_batch.schema(),
        _ => return AssociatedStatementNotPreparedSnafu.fail(),
    };

    if rec_number == 0 {
        match field {
            DescField::Count => {
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, desc.desc_count);
                }
                Ok(())
            }
            DescField::ArrayStatusPtr => {
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut *mut u16, desc.array_status_ptr);
                }
                Ok(())
            }
            DescField::RowsProcessedPtr => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut *mut sql::ULen,
                        desc.rows_processed_ptr,
                    );
                }
                Ok(())
            }
            _ => {
                tracing::warn!("get_desc_field: unsupported IRD header field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        }
    } else {
        let col_idx = (rec_number as usize) - 1;
        if col_idx >= schema.fields().len() {
            return crate::api::error::NoMoreDataSnafu.fail();
        }

        let arrow_field = schema.field(col_idx);
        let dbc = guard.conn()?;
        let numeric_settings = dbc.connection.lock().numeric_settings;

        match compute_ird_field(arrow_field, field, &numeric_settings)? {
            IrdFieldValue::SmallInt(v) => {
                unsafe { std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, v) };
            }
            IrdFieldValue::Integer(v) => {
                unsafe { std::ptr::write_unaligned(value_ptr as *mut sql::Integer, v) };
            }
            IrdFieldValue::Len(v) => {
                unsafe { std::ptr::write_unaligned(value_ptr as *mut sql::Len, v) };
            }
            IrdFieldValue::Str(s) => {
                write_string_bytes_i32::<E>(
                    s,
                    value_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
            }
        }
        Ok(())
    }
}

/// Get a descriptor record (composite of multiple fields).
#[allow(clippy::too_many_arguments)]
pub fn get_desc_rec<E: OdbcEncoding>(
    desc_handle: sql::Handle,
    rec_number: sql::SmallInt,
    name: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    type_ptr: *mut sql::SmallInt,
    sub_type_ptr: *mut sql::SmallInt,
    length_ptr: *mut sql::Len,
    precision_ptr: *mut sql::SmallInt,
    scale_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    if rec_number <= 0 {
        return crate::api::error::InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    let access = desc_from_handle(desc_handle)?;

    match access {
        DescriptorAccess::Implicit { guard, kind } => {
            let inner = guard.inner.lock();
            if inner.state.as_ref().is_need_data() {
                return crate::api::error::InvalidDuringDaeSnafu.fail();
            }
            match kind {
                DescriptorKind::Ird => get_ird_rec::<E>(
                    rec_number,
                    name,
                    buffer_length,
                    string_length_ptr,
                    type_ptr,
                    sub_type_ptr,
                    length_ptr,
                    precision_ptr,
                    scale_ptr,
                    nullable_ptr,
                    &guard,
                    &inner.state,
                    warnings,
                ),
                DescriptorKind::Ard => get_ard_rec::<E>(
                    &inner.ard,
                    rec_number,
                    name,
                    buffer_length,
                    string_length_ptr,
                    type_ptr,
                    sub_type_ptr,
                    length_ptr,
                    precision_ptr,
                    scale_ptr,
                    nullable_ptr,
                    warnings,
                ),
                DescriptorKind::Apd => get_apd_rec::<E>(
                    &inner.apd,
                    rec_number,
                    name,
                    buffer_length,
                    string_length_ptr,
                    type_ptr,
                    sub_type_ptr,
                    length_ptr,
                    precision_ptr,
                    scale_ptr,
                    nullable_ptr,
                    warnings,
                ),
                DescriptorKind::Ipd => get_ipd_rec::<E>(
                    &inner.ipd,
                    rec_number,
                    name,
                    buffer_length,
                    string_length_ptr,
                    type_ptr,
                    sub_type_ptr,
                    length_ptr,
                    precision_ptr,
                    scale_ptr,
                    nullable_ptr,
                    warnings,
                ),
            }
        }
        DescriptorAccess::Explicit { desc } => {
            let desc = desc.lock();
            get_ard_rec::<E>(
                &desc,
                rec_number,
                name,
                buffer_length,
                string_length_ptr,
                type_ptr,
                sub_type_ptr,
                length_ptr,
                precision_ptr,
                scale_ptr,
                nullable_ptr,
                warnings,
            )
        }
    }
}

struct DescRecValues<'a> {
    name: &'a str,
    type_value: sql::SmallInt,
    concise_type: sql::SmallInt,
    octet_length: sql::Len,
    precision: sql::SmallInt,
    scale: sql::SmallInt,
    nullable: sql::SmallInt,
}

#[allow(clippy::too_many_arguments)]
fn write_desc_rec<E: OdbcEncoding>(
    values: &DescRecValues<'_>,
    name_buf: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    type_ptr: *mut sql::SmallInt,
    sub_type_ptr: *mut sql::SmallInt,
    length_ptr: *mut sql::Len,
    precision_ptr: *mut sql::SmallInt,
    scale_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) {
    if !name_buf.is_null() || !string_length_ptr.is_null() {
        write_string_chars::<E>(
            values.name,
            name_buf,
            buffer_length,
            string_length_ptr,
            Some(warnings),
        );
    }
    if !type_ptr.is_null() {
        unsafe { std::ptr::write_unaligned(type_ptr, values.type_value) };
    }
    if !sub_type_ptr.is_null() {
        unsafe {
            std::ptr::write_unaligned(sub_type_ptr, datetime_interval_code(values.concise_type))
        };
    }
    if !length_ptr.is_null() {
        unsafe { std::ptr::write_unaligned(length_ptr, values.octet_length) };
    }
    if !precision_ptr.is_null() {
        unsafe { std::ptr::write_unaligned(precision_ptr, values.precision) };
    }
    if !scale_ptr.is_null() {
        unsafe { std::ptr::write_unaligned(scale_ptr, values.scale) };
    }
    if !nullable_ptr.is_null() {
        unsafe { std::ptr::write_unaligned(nullable_ptr, values.nullable) };
    }
}

#[allow(clippy::too_many_arguments)]
fn get_ird_rec<E: OdbcEncoding>(
    rec_number: sql::SmallInt,
    name: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    type_ptr: *mut sql::SmallInt,
    sub_type_ptr: *mut sql::SmallInt,
    length_ptr: *mut sql::Len,
    precision_ptr: *mut sql::SmallInt,
    scale_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
    guard: &HandleGuard<Statement>,
    state: &State<StatementState>,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let schema = match state.as_ref() {
        StatementState::Prepared { schema } => schema.clone(),
        StatementState::QueryExecuted { reader, .. } => reader.schema(),
        StatementState::Fetching { record_batch, .. } => record_batch.schema(),
        _ => return AssociatedStatementNotPreparedSnafu.fail(),
    };

    let col_idx = (rec_number as usize) - 1;
    if col_idx >= schema.fields().len() {
        return crate::api::error::NoMoreDataSnafu.fail();
    }

    let arrow_field = schema.field(col_idx);
    let dbc = guard.conn()?;
    let numeric_settings = dbc.connection.lock().numeric_settings;

    let values = DescRecValues {
        name: compute_ird_name(arrow_field),
        type_value: compute_ird_verbose_type(arrow_field, &numeric_settings)?,
        concise_type: compute_ird_concise_type(arrow_field, &numeric_settings)?,
        octet_length: compute_ird_octet_length(arrow_field, &numeric_settings)?,
        precision: compute_ird_precision(arrow_field, &numeric_settings)?,
        scale: compute_ird_scale(arrow_field, &numeric_settings)?,
        nullable: compute_ird_nullable(arrow_field),
    };
    write_desc_rec::<E>(
        &values,
        name,
        buffer_length,
        string_length_ptr,
        type_ptr,
        sub_type_ptr,
        length_ptr,
        precision_ptr,
        scale_ptr,
        nullable_ptr,
        warnings,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn get_ard_rec<E: OdbcEncoding>(
    desc: &crate::api::ArdDescriptor,
    rec_number: sql::SmallInt,
    name: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    type_ptr: *mut sql::SmallInt,
    sub_type_ptr: *mut sql::SmallInt,
    length_ptr: *mut sql::Len,
    precision_ptr: *mut sql::SmallInt,
    scale_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let binding = desc.bindings.get(&(rec_number as u16)).ok_or_else(|| {
        crate::api::error::OdbcError::NoMoreData {
            location: snafu::location!(),
        }
    })?;

    let concise_type = binding.target_type as sql::SmallInt;
    let values = DescRecValues {
        name: "",
        type_value: concise_type,
        concise_type,
        octet_length: binding.buffer_length,
        precision: 0,
        scale: 0,
        nullable: 0,
    };
    write_desc_rec::<E>(
        &values,
        name,
        buffer_length,
        string_length_ptr,
        type_ptr,
        sub_type_ptr,
        length_ptr,
        precision_ptr,
        scale_ptr,
        nullable_ptr,
        warnings,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn get_apd_rec<E: OdbcEncoding>(
    desc: &crate::api::ApdDescriptor,
    rec_number: sql::SmallInt,
    name: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    type_ptr: *mut sql::SmallInt,
    sub_type_ptr: *mut sql::SmallInt,
    length_ptr: *mut sql::Len,
    precision_ptr: *mut sql::SmallInt,
    scale_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let record = desc.records.get(&(rec_number as u16)).ok_or_else(|| {
        crate::api::error::OdbcError::NoMoreData {
            location: snafu::location!(),
        }
    })?;

    let concise_type = record.value_type as sql::SmallInt;
    let values = DescRecValues {
        name: "",
        type_value: concise_type,
        concise_type,
        octet_length: record.buffer_length,
        precision: 0,
        scale: 0,
        nullable: 0,
    };
    write_desc_rec::<E>(
        &values,
        name,
        buffer_length,
        string_length_ptr,
        type_ptr,
        sub_type_ptr,
        length_ptr,
        precision_ptr,
        scale_ptr,
        nullable_ptr,
        warnings,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn get_ipd_rec<E: OdbcEncoding>(
    desc: &crate::api::IpdDescriptor,
    rec_number: sql::SmallInt,
    name: *mut E::Char,
    buffer_length: sql::SmallInt,
    string_length_ptr: *mut sql::SmallInt,
    type_ptr: *mut sql::SmallInt,
    sub_type_ptr: *mut sql::SmallInt,
    length_ptr: *mut sql::Len,
    precision_ptr: *mut sql::SmallInt,
    scale_ptr: *mut sql::SmallInt,
    nullable_ptr: *mut sql::SmallInt,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    let record = desc.records.get(&(rec_number as u16)).ok_or_else(|| {
        crate::api::error::OdbcError::NoMoreData {
            location: snafu::location!(),
        }
    })?;

    let concise_type = record.sql_data_type.0;
    let precision = if record.column_size <= i16::MAX as sql::ULen {
        record.column_size as sql::SmallInt
    } else {
        i16::MAX
    };
    let values = DescRecValues {
        name: "",
        type_value: concise_type,
        concise_type,
        octet_length: ipd_octet_length(record.sql_data_type, record.column_size),
        precision,
        scale: record.decimal_digits,
        nullable: record.nullable,
    };
    write_desc_rec::<E>(
        &values,
        name,
        buffer_length,
        string_length_ptr,
        type_ptr,
        sub_type_ptr,
        length_ptr,
        precision_ptr,
        scale_ptr,
        nullable_ptr,
        warnings,
    );
    Ok(())
}

fn datetime_interval_code(concise_type: sql::SmallInt) -> sql::SmallInt {
    match sql::SqlDataType(concise_type) {
        sql::SqlDataType::DATE => 1,      // SQL_CODE_DATE
        sql::SqlDataType::TIME => 2,      // SQL_CODE_TIME
        sql::SqlDataType::TIMESTAMP => 3, // SQL_CODE_TIMESTAMP
        _ => 0,
    }
}

/// Inverse of [`datetime_interval_code`]: for a `SQL_DESC_TYPE` of
/// `SQL_DATETIME`, `SQLSetDescRec`'s `SubType` argument carries the
/// `SQL_DESC_DATETIME_INTERVAL_CODE`, which selects the concise datetime type
/// the driver stores internally. Returns `None` for a code the driver does not
/// model (mirrors `datetime_interval_code`, which only covers date/time/timestamp).
fn concise_datetime_type(sub_type: sql::SmallInt) -> Option<sql::SmallInt> {
    match sub_type {
        1 => Some(sql::SqlDataType::DATE.0), // SQL_CODE_DATE      -> SQL_TYPE_DATE
        2 => Some(sql::SqlDataType::TIME.0), // SQL_CODE_TIME      -> SQL_TYPE_TIME
        3 => Some(sql::SqlDataType::TIMESTAMP.0), // SQL_CODE_TIMESTAMP -> SQL_TYPE_TIMESTAMP
        _ => None,
    }
}

fn verbose_type_from_concise(concise_type: sql::SmallInt) -> sql::SmallInt {
    if datetime_interval_code(concise_type) != 0 {
        9 // SQL_DATETIME
    } else {
        concise_type
    }
}

fn ipd_octet_length(sql_type: sql::SqlDataType, column_size: sql::ULen) -> sql::Len {
    match sql_type {
        sql::SqlDataType::EXT_BIT => 1,
        sql::SqlDataType::EXT_TINY_INT => 1,
        sql::SqlDataType::SMALLINT => 2,
        sql::SqlDataType::INTEGER => 4,
        sql::SqlDataType::EXT_BIG_INT => 8,
        sql::SqlDataType::REAL => 4,
        sql::SqlDataType::FLOAT | sql::SqlDataType::DOUBLE => 8,
        sql::SqlDataType::DATE => 6,       // sizeof(SQL_DATE_STRUCT)
        sql::SqlDataType::TIME => 6,       // sizeof(SQL_TIME_STRUCT)
        sql::SqlDataType::TIMESTAMP => 16, // sizeof(SQL_TIMESTAMP_STRUCT)
        _ => column_size as sql::Len,
    }
}

/// Set a descriptor field value
pub fn set_desc_field<E: OdbcEncoding>(
    desc_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!(
        "set_desc_field: desc_handle={:?}, rec_number={}, field_identifier={}",
        desc_handle,
        rec_number,
        field_identifier
    );

    if rec_number < 0 {
        tracing::error!("set_desc_field: invalid negative rec_number {}", rec_number);
        return crate::api::error::InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    let field = DescField::try_from(field_identifier)?;

    if field == DescField::AllocType {
        return crate::api::error::InvalidDescriptorFieldIdSnafu {
            field_id: field_identifier,
        }
        .fail();
    }

    let access = desc_from_handle(desc_handle)?;

    match access {
        DescriptorAccess::Implicit { guard, kind } => {
            let mut inner = guard.inner.lock();
            if inner.state.as_ref().is_need_data() {
                return crate::api::error::InvalidDuringDaeSnafu.fail();
            }
            match kind {
                DescriptorKind::Ard => set_ard_field(&mut inner.ard, rec_number, field, value_ptr),
                DescriptorKind::Ird => set_ird_field(&mut inner.ird, rec_number, field, value_ptr),
                DescriptorKind::Apd => set_apd_field(&mut inner.apd, rec_number, field, value_ptr),
                DescriptorKind::Ipd => {
                    set_ipd_field::<E>(&mut inner.ipd, rec_number, field, value_ptr, buffer_length)
                }
            }
        }
        DescriptorAccess::Explicit { desc } => {
            let mut desc = desc.lock();
            set_ard_field(&mut desc, rec_number, field, value_ptr)
        }
    }
}

fn set_ard_field(
    desc: &mut crate::api::ArdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
) -> OdbcResult<()> {
    if rec_number == 0 {
        match field {
            DescField::Count => {
                let count = value_ptr as sql::SmallInt;
                if count < 0 {
                    tracing::error!("set_desc_field: invalid negative count {}", count);
                    return crate::api::error::InvalidDescriptorIndexSnafu { number: count }.fail();
                }
                desc.set_desc_count(count);
                Ok(())
            }
            DescField::ArraySize => {
                let size = value_ptr as usize;
                tracing::debug!("set_desc_field: ARD ArraySize = {}", size);
                if size == 0 {
                    tracing::error!(
                        "set_desc_field: invalid ARD ArraySize {}, must be >= 1",
                        size
                    );
                    return crate::api::error::InvalidDescriptorIndexSnafu { number: 0i16 }.fail();
                }
                desc.array_size = size;
                Ok(())
            }
            DescField::BindType => {
                let bind_type = value_ptr as usize;
                tracing::debug!("set_desc_field: ARD BindType = {}", bind_type);
                desc.bind_type = bind_type;
                Ok(())
            }
            DescField::BindOffsetPtr => {
                let ptr = value_ptr as *mut sql::Len;
                tracing::debug!("set_desc_field: ARD BindOffsetPtr = {:?}", ptr);
                desc.bind_offset_ptr = ptr;
                Ok(())
            }
            _ => {
                tracing::warn!("set_desc_field: unsupported ARD header field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        }
    } else {
        let column_number = rec_number as u16;

        // Per ODBC, setting any field on a bound ARD record other than the
        // deferred pointer fields (DATA_PTR, OCTET_LENGTH_PTR, INDICATOR_PTR)
        // runs a consistency check that unbinds the record — SQL_DESC_DATA_PTR
        // is reset to a null pointer. Only applied when the set succeeds.
        let is_deferred = matches!(
            field,
            DescField::DataPtr | DescField::OctetLengthPtr | DescField::IndicatorPtr
        );

        let result = match field {
            DescField::Type | DescField::ConciseType => {
                let raw = value_ptr as i16;
                let c_type = CDataType::try_from(raw)?;
                tracing::debug!(
                    "set_desc_field: setting target_type={c_type:?} on record {column_number}",
                );
                let binding = desc.bindings.entry(column_number).or_default();
                binding.target_type = c_type;
                Ok(())
            }
            DescField::Precision => {
                let precision = value_ptr as i16;
                if !(0..=38).contains(&precision) {
                    tracing::error!(
                        "set_desc_field: precision {precision} out of valid range 0..=38"
                    );
                    return crate::api::error::InvalidPrecisionOrScaleSnafu {
                        reason: format!(
                            "SQL_DESC_PRECISION value {precision} is out of valid range (0-38)"
                        ),
                    }
                    .fail();
                }
                tracing::debug!(
                    "set_desc_field: setting precision={precision} on record {column_number}"
                );
                let binding = desc.bindings.entry(column_number).or_default();
                binding.precision = Some(precision);
                Ok(())
            }
            DescField::Scale => {
                let scale = value_ptr as i16;
                if scale < i8::MIN as i16 || scale > i8::MAX as i16 {
                    tracing::error!("set_desc_field: scale {scale} out of valid range for i8");
                    return crate::api::error::InvalidPrecisionOrScaleSnafu {
                        reason: format!(
                            "SQL_DESC_SCALE value {scale} is out of valid range ({min}..={max})",
                            min = i8::MIN,
                            max = i8::MAX,
                        ),
                    }
                    .fail();
                }
                tracing::debug!("set_desc_field: setting scale={scale} on record {column_number}");
                let binding = desc.bindings.entry(column_number).or_default();
                binding.scale = Some(scale);
                Ok(())
            }
            DescField::DataPtr => {
                tracing::debug!("set_desc_field: setting data_ptr on record {column_number}");
                let binding = desc.bindings.entry(column_number).or_default();
                binding.target_value_ptr = value_ptr;
                Ok(())
            }
            DescField::OctetLength => {
                let length = value_ptr as sql::Len;
                tracing::debug!(
                    "set_desc_field: setting buffer_length={length} on record {column_number}"
                );
                let binding = desc.bindings.entry(column_number).or_default();
                binding.buffer_length = length;
                Ok(())
            }
            DescField::IndicatorPtr => {
                let ptr = value_ptr as *mut sql::Len;
                tracing::debug!("set_desc_field: setting indicator_ptr on record {column_number}");
                let binding = desc.bindings.entry(column_number).or_default();
                binding.indicator_ptr = ptr;
                Ok(())
            }
            DescField::OctetLengthPtr => {
                let ptr = value_ptr as *mut sql::Len;
                tracing::debug!(
                    "set_desc_field: setting octet_length_ptr on record {column_number}"
                );
                let binding = desc.bindings.entry(column_number).or_default();
                binding.octet_length_ptr = ptr;
                Ok(())
            }
            DescField::DatetimeIntervalPrecision => {
                let dip = value_ptr as i16;
                if !(0..=9).contains(&dip) {
                    tracing::error!(
                        "set_desc_field: datetime_interval_precision {dip} out of valid range 0..=9"
                    );
                    return crate::api::error::InvalidPrecisionOrScaleSnafu {
                        reason: format!(
                            "SQL_DESC_DATETIME_INTERVAL_PRECISION value {dip} is out of valid range (0-9)"
                        ),
                    }
                    .fail();
                }
                tracing::debug!(
                    "set_desc_field: setting datetime_interval_precision={dip} on record {column_number}"
                );
                let binding = desc.bindings.entry(column_number).or_default();
                binding.datetime_interval_precision = Some(dip);
                Ok(())
            }
            DescField::Length => {
                let length = value_ptr as sql::ULen;
                tracing::debug!(
                    "set_desc_field: setting length={length} on record {column_number}"
                );
                let binding = desc.bindings.entry(column_number).or_default();
                binding.length = length;
                Ok(())
            }
            _ => {
                tracing::warn!("set_desc_field: unsupported ARD record field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        };

        if result.is_ok()
            && !is_deferred
            && let Some(binding) = desc.bindings.get_mut(&column_number)
        {
            binding.target_value_ptr = std::ptr::null_mut();
        }
        result
    }
}

fn set_ird_field(
    desc: &mut crate::api::IrdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
) -> OdbcResult<()> {
    if rec_number == 0 {
        match field {
            DescField::ArrayStatusPtr => {
                let ptr = value_ptr as *mut u16;
                tracing::debug!("set_desc_field: IRD ArrayStatusPtr = {:?}", ptr);
                desc.array_status_ptr = ptr;
                Ok(())
            }
            DescField::RowsProcessedPtr => {
                let ptr = value_ptr as *mut sql::ULen;
                tracing::debug!("set_desc_field: IRD RowsProcessedPtr = {:?}", ptr);
                desc.rows_processed_ptr = ptr;
                Ok(())
            }
            _ => {
                tracing::warn!("set_desc_field: IRD header field {:?} is read-only", field);
                crate::api::error::CannotModifyIrdSnafu.fail()
            }
        }
    } else {
        tracing::warn!(
            "set_desc_field: IRD record fields are read-only (rec={})",
            rec_number
        );
        crate::api::error::CannotModifyIrdSnafu.fail()
    }
}

// =============================================================================
// APD — Application Parameter Descriptor
// =============================================================================

fn get_apd_field(
    desc: &crate::api::ApdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
) -> OdbcResult<()> {
    // Per ODBC spec: "If the FieldIdentifier argument indicates a header field,
    // RecNumber is ignored." Handle header fields first regardless of rec_number.
    match field {
        DescField::Count => {
            let count = desc.desc_count();
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, count as sql::SmallInt);
            }
            return Ok(());
        }
        DescField::ArraySize => {
            unsafe {
                std::ptr::write_unaligned(
                    value_ptr as *mut sql::ULen,
                    desc.array_size as sql::ULen,
                );
            }
            return Ok(());
        }
        DescField::BindType => {
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut sql::ULen, desc.bind_type);
            }
            return Ok(());
        }
        DescField::BindOffsetPtr => {
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut *mut sql::Len, desc.bind_offset_ptr);
            }
            return Ok(());
        }
        DescField::ArrayStatusPtr => {
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut *mut u16, desc.array_status_ptr);
            }
            return Ok(());
        }
        _ => {}
    }

    if rec_number == 0 {
        tracing::warn!(
            "get_desc_field: unsupported APD field {:?} for record 0",
            field
        );
        return crate::api::error::InvalidDescriptorFieldIdSnafu {
            field_id: field as i16,
        }
        .fail();
    }

    let param_number = rec_number as u16;
    let record = match desc.records.get(&param_number) {
        Some(r) => r,
        None => return crate::api::error::NoMoreDataSnafu.fail(),
    };

    match field {
        DescField::Type | DescField::ConciseType => {
            unsafe {
                std::ptr::write_unaligned(
                    value_ptr as *mut sql::SmallInt,
                    record.value_type as sql::SmallInt,
                );
            }
            Ok(())
        }
        DescField::DataPtr => {
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut sql::Pointer, record.data_ptr);
            }
            Ok(())
        }
        DescField::OctetLength => {
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut sql::Len, record.buffer_length);
            }
            Ok(())
        }
        DescField::IndicatorPtr | DescField::OctetLengthPtr => {
            unsafe {
                std::ptr::write_unaligned(
                    value_ptr as *mut *mut sql::Len,
                    record.str_len_or_ind_ptr,
                );
            }
            Ok(())
        }
        _ => {
            tracing::warn!("get_desc_field: unsupported APD record field {:?}", field);
            crate::api::error::InvalidDescriptorFieldIdSnafu {
                field_id: field as i16,
            }
            .fail()
        }
    }
}

fn set_apd_field(
    desc: &mut crate::api::ApdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
) -> OdbcResult<()> {
    if rec_number == 0 {
        match field {
            DescField::Count => {
                let count = value_ptr as sql::SmallInt;
                if count < 0 {
                    return crate::api::error::InvalidDescriptorIndexSnafu { number: count }.fail();
                }
                if count == 0 {
                    desc.clear();
                } else {
                    desc.records.retain(|&k, _| k <= count as u16);
                }
                Ok(())
            }
            DescField::ArraySize => {
                let size = value_ptr as usize;
                if size == 0 {
                    return crate::api::error::InvalidDescriptorIndexSnafu { number: 0i16 }.fail();
                }
                desc.array_size = size;
                Ok(())
            }
            DescField::BindType => {
                desc.bind_type = value_ptr as usize;
                Ok(())
            }
            DescField::BindOffsetPtr => {
                desc.bind_offset_ptr = value_ptr as *mut sql::Len;
                Ok(())
            }
            DescField::ArrayStatusPtr => {
                desc.array_status_ptr = value_ptr as *mut u16;
                Ok(())
            }
            _ => {
                tracing::warn!("set_desc_field: unsupported APD header field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        }
    } else {
        let param_number = rec_number as u16;
        let record = desc.records.entry(param_number).or_default();

        match field {
            DescField::Type | DescField::ConciseType => {
                let raw = value_ptr as i16;
                let c_type = CDataType::try_from(raw).map_err(|unknown| {
                    tracing::error!("set_desc_field: unknown C data type discriminant {unknown}");
                    crate::api::error::OdbcError::InvalidApplicationBufferType {
                        location: snafu::location!(),
                    }
                })?;
                record.value_type = c_type;
                Ok(())
            }
            DescField::DataPtr => {
                record.data_ptr = value_ptr;
                Ok(())
            }
            DescField::OctetLength => {
                record.buffer_length = value_ptr as sql::Len;
                Ok(())
            }
            DescField::IndicatorPtr | DescField::OctetLengthPtr => {
                record.str_len_or_ind_ptr = value_ptr as *mut sql::Len;
                Ok(())
            }
            _ => {
                tracing::warn!("set_desc_field: unsupported APD record field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        }
    }
}

// =============================================================================
// IPD — Implementation Parameter Descriptor
// =============================================================================

fn get_ipd_field<E: OdbcEncoding>(
    desc: &crate::api::IpdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
    warnings: &mut Warnings,
) -> OdbcResult<()> {
    // Per ODBC spec: header fields ignore RecNumber.
    match field {
        DescField::Count => {
            let count = desc.desc_count();
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, count as sql::SmallInt);
            }
            return Ok(());
        }
        DescField::ArrayStatusPtr => {
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut *mut u16, desc.array_status_ptr);
            }
            return Ok(());
        }
        DescField::RowsProcessedPtr => {
            unsafe {
                std::ptr::write_unaligned(
                    value_ptr as *mut *mut sql::ULen,
                    desc.rows_processed_ptr,
                );
            }
            return Ok(());
        }
        _ => {}
    }

    if rec_number == 0 {
        tracing::warn!(
            "get_desc_field: unsupported IPD field {:?} for record 0",
            field
        );
        return crate::api::error::InvalidRecordNumberSnafu { number: 0i16 }.fail();
    }

    {
        let param_number = rec_number as u16;
        let record = match desc.records.get(&param_number) {
            Some(r) => r,
            None => return crate::api::error::NoMoreDataSnafu.fail(),
        };

        match field {
            DescField::Type | DescField::ConciseType => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        record.sql_data_type.0,
                    );
                }
                Ok(())
            }
            DescField::Precision => {
                let precision = if record.column_size <= i16::MAX as sql::ULen {
                    record.column_size as sql::SmallInt
                } else {
                    i16::MAX
                };
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, precision);
                }
                Ok(())
            }
            DescField::Scale => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        record.decimal_digits,
                    );
                }
                Ok(())
            }
            DescField::ParameterType => {
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, record.direction);
                }
                Ok(())
            }
            DescField::Nullable => {
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, record.nullable);
                }
                Ok(())
            }
            DescField::Name => {
                // SQL_DESC_NAME — the app-set parameter name, or empty string
                // when unnamed. Byte-length semantics, encoding per the C API.
                let name = record.name.as_deref().unwrap_or("");
                write_string_bytes_i32::<E>(
                    name,
                    value_ptr as *mut E::Char,
                    buffer_length,
                    string_length_ptr,
                    Some(warnings),
                );
                Ok(())
            }
            DescField::Unnamed => {
                // SQL_NAMED (0) when a name has been set, else SQL_UNNAMED (1).
                let unnamed: sql::SmallInt = if record.name.is_some() { 0 } else { 1 };
                unsafe {
                    std::ptr::write_unaligned(value_ptr as *mut sql::SmallInt, unnamed);
                }
                Ok(())
            }
            _ => {
                tracing::warn!("get_desc_field: unsupported IPD record field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        }
    }
}

fn set_ipd_field<E: OdbcEncoding>(
    desc: &mut crate::api::IpdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
    buffer_length: sql::Integer,
) -> OdbcResult<()> {
    if rec_number == 0 {
        match field {
            DescField::ArrayStatusPtr => {
                desc.array_status_ptr = value_ptr as *mut u16;
                Ok(())
            }
            DescField::RowsProcessedPtr => {
                desc.rows_processed_ptr = value_ptr as *mut sql::ULen;
                Ok(())
            }
            // Record fields require a 1-based record number; RecNumber 0 → 07009.
            _ => {
                tracing::warn!(
                    "set_desc_field: IPD record field {:?} requires RecNumber >= 1",
                    field
                );
                crate::api::error::InvalidDescriptorIndexSnafu { number: 0i16 }.fail()
            }
        }
    } else {
        let param_number = rec_number as u16;
        let record = desc.records.entry(param_number).or_default();

        match field {
            DescField::Type | DescField::ConciseType => {
                let raw_type = value_ptr as i16;
                crate::api::SqlType::try_from(raw_type)?;
                record.sql_data_type = sql::SqlDataType(raw_type);
                Ok(())
            }
            DescField::Precision => {
                record.column_size = value_ptr as sql::ULen;
                Ok(())
            }
            DescField::Scale => {
                record.decimal_digits = value_ptr as sql::SmallInt;
                Ok(())
            }
            DescField::ParameterType => {
                // `ParamDirection::try_from` rejects out-of-range values with
                // `InvalidParameterType` (HY105).
                let direction = value_ptr as sql::SmallInt;
                crate::api::ParamDirection::try_from(direction)?;
                record.direction = direction;
                Ok(())
            }
            DescField::Nullable => {
                record.nullable = value_ptr as sql::SmallInt;
                Ok(())
            }
            DescField::Name => {
                // SQL_DESC_NAME is a character field; `read_string` decodes per
                // the C API's encoding and rejects a negative BufferLength that
                // isn't SQL_NTS with HY090.
                let name = E::read_string(value_ptr as *const E::Char, buffer_length)?;
                tracing::debug!("set_desc_field: IPD param {param_number} name={name:?}");
                record.name = Some(name);
                Ok(())
            }
            DescField::Unnamed => {
                // Applications may only set SQL_DESC_UNNAMED to SQL_UNNAMED (1),
                // which clears the name; SQL_NAMED (0) is driver-owned → HY092.
                const SQL_UNNAMED: sql::SmallInt = 1;
                let value = value_ptr as sql::SmallInt;
                if value == SQL_UNNAMED {
                    record.name = None;
                    Ok(())
                } else {
                    crate::api::error::CannotSetUnnamedToNamedSnafu.fail()
                }
            }
            _ => {
                tracing::warn!("set_desc_field: unsupported IPD record field {:?}", field);
                crate::api::error::InvalidDescriptorFieldIdSnafu {
                    field_id: field as i16,
                }
                .fail()
            }
        }
    }
}

fn is_valid_ard_record_field(field: DescField) -> bool {
    matches!(
        field,
        DescField::Type
            | DescField::ConciseType
            | DescField::OctetLength
            | DescField::Length
            | DescField::DataPtr
            | DescField::IndicatorPtr
            | DescField::OctetLengthPtr
            | DescField::DatetimeIntervalPrecision
            | DescField::DatetimeIntervalCode
            | DescField::Precision
            | DescField::Scale
    )
}

// =============================================================================
// SQLSetDescRec
// =============================================================================

#[allow(clippy::too_many_arguments)]
pub fn set_desc_rec(
    desc_handle: sql::Handle,
    rec_number: sql::SmallInt,
    type_: sql::SmallInt,
    sub_type: sql::SmallInt,
    length: sql::Len,
    precision: sql::SmallInt,
    scale: sql::SmallInt,
    data_ptr: sql::Pointer,
    octet_length_ptr: *mut sql::Len,
    indicator_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    tracing::debug!(
        "set_desc_rec: desc_handle={:?}, rec_number={}, type={}, length={}",
        desc_handle,
        rec_number,
        type_,
        length,
    );

    if rec_number < 0 {
        return crate::api::error::InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    // Per SQLSetDescRec, when Type is SQL_DATETIME the SubType arg is the
    // SQL_DESC_DATETIME_INTERVAL_CODE; resolve it to the concise datetime type
    // the driver stores (so TYPE/CONCISE_TYPE/DATETIME_INTERVAL_CODE read back
    // correctly), mirroring the CONCISE_TYPE handling in `set_desc_field`.
    const SQL_DATETIME: sql::SmallInt = 9;
    let type_ = if type_ == SQL_DATETIME {
        concise_datetime_type(sub_type).ok_or_else(|| {
            crate::api::error::InconsistentDescriptorInfoSnafu {
                reason: format!("invalid datetime SubType {sub_type} for SQL_DATETIME"),
            }
            .build()
        })?
    } else {
        type_
    };

    let access = desc_from_handle(desc_handle)?;

    match access {
        DescriptorAccess::Implicit { guard, kind } => {
            let mut inner = guard.inner.lock();
            if inner.state.as_ref().is_need_data() {
                return crate::api::error::InvalidDuringDaeSnafu.fail();
            }
            match kind {
                DescriptorKind::Ard => set_ard_rec(
                    &mut inner.ard,
                    rec_number,
                    type_,
                    length,
                    precision,
                    scale,
                    data_ptr,
                    octet_length_ptr,
                    indicator_ptr,
                ),
                DescriptorKind::Ird => crate::api::error::CannotModifyIrdSnafu.fail(),
                DescriptorKind::Apd => set_apd_rec(
                    &mut inner.apd,
                    rec_number,
                    type_,
                    length,
                    data_ptr,
                    octet_length_ptr,
                    indicator_ptr,
                ),
                DescriptorKind::Ipd => {
                    set_ipd_rec(&mut inner.ipd, rec_number, type_, precision, scale)
                }
            }
        }
        DescriptorAccess::Explicit { desc } => {
            let mut desc = desc.lock();
            set_ard_rec(
                &mut desc,
                rec_number,
                type_,
                length,
                precision,
                scale,
                data_ptr,
                octet_length_ptr,
                indicator_ptr,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn set_ard_rec(
    desc: &mut crate::api::ArdDescriptor,
    rec_number: sql::SmallInt,
    type_: sql::SmallInt,
    length: sql::Len,
    precision: sql::SmallInt,
    scale: sql::SmallInt,
    data_ptr: sql::Pointer,
    octet_length_ptr: *mut sql::Len,
    indicator_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    if rec_number == 0 {
        return crate::api::error::InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    let c_type = CDataType::try_from(type_).map_err(|_| {
        crate::api::error::InconsistentDescriptorInfoSnafu {
            reason: format!("invalid C data type {type_}"),
        }
        .build()
    })?;

    let column_number = rec_number as u16;
    let binding = desc.bindings.entry(column_number).or_default();
    binding.target_type = c_type;
    binding.buffer_length = length;
    binding.target_value_ptr = data_ptr;
    binding.octet_length_ptr = octet_length_ptr;
    binding.indicator_ptr = indicator_ptr;
    // SQLSetDescRec sets SQL_DESC_PRECISION / SQL_DESC_SCALE directly — a passed
    // 0 must overwrite any prior value (matches set_ard_field; None stays
    // reserved for "never set").
    binding.precision = Some(precision);
    binding.scale = Some(scale);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn set_apd_rec(
    desc: &mut crate::api::ApdDescriptor,
    rec_number: sql::SmallInt,
    type_: sql::SmallInt,
    length: sql::Len,
    data_ptr: sql::Pointer,
    octet_length_ptr: *mut sql::Len,
    indicator_ptr: *mut sql::Len,
) -> OdbcResult<()> {
    if rec_number == 0 {
        return crate::api::error::InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    let c_type = CDataType::try_from(type_).map_err(|_| {
        crate::api::error::InconsistentDescriptorInfoSnafu {
            reason: format!("invalid C data type {type_}"),
        }
        .build()
    })?;

    let param_number = rec_number as u16;
    let record = desc.records.entry(param_number).or_default();
    record.value_type = c_type;
    record.buffer_length = length;
    record.data_ptr = data_ptr;
    record.str_len_or_ind_ptr = if !indicator_ptr.is_null() {
        indicator_ptr
    } else {
        octet_length_ptr
    };

    Ok(())
}

fn set_ipd_rec(
    desc: &mut crate::api::IpdDescriptor,
    rec_number: sql::SmallInt,
    type_: sql::SmallInt,
    precision: sql::SmallInt,
    scale: sql::SmallInt,
) -> OdbcResult<()> {
    if rec_number == 0 {
        return crate::api::error::InvalidRecordNumberSnafu { number: rec_number }.fail();
    }

    crate::api::SqlType::try_from(type_).map_err(|_| {
        crate::api::error::InconsistentDescriptorInfoSnafu {
            reason: format!("invalid SQL data type {type_}"),
        }
        .build()
    })?;

    let param_number = rec_number as u16;
    let record = desc.records.entry(param_number).or_default();
    record.sql_data_type = sql::SqlDataType(type_);
    record.column_size = precision as sql::ULen;
    record.decimal_digits = scale;

    Ok(())
}

// =============================================================================
// SQLCopyDesc
// =============================================================================

pub fn copy_desc(source_handle: sql::Handle, target_handle: sql::Handle) -> OdbcResult<()> {
    tracing::debug!(
        "copy_desc: source={:?}, target={:?}",
        source_handle,
        target_handle,
    );

    let source_access = desc_from_handle(source_handle)?;

    let source_data = match source_access {
        DescriptorAccess::Implicit { guard, kind } => {
            let inner = guard.inner.lock();
            if inner.state.as_ref().is_need_data() {
                return crate::api::error::InvalidDuringDaeSnafu.fail();
            }
            match kind {
                DescriptorKind::Ard => CopyData::Ard(inner.ard.clone_for_copy()),
                DescriptorKind::Apd => CopyData::Apd(inner.apd.clone_for_copy()),
                DescriptorKind::Ird => {
                    if matches!(inner.state.as_ref(), StatementState::Created) {
                        return crate::api::error::AssociatedStatementNotPreparedSnafu.fail();
                    }
                    return crate::api::error::InconsistentDescriptorInfoSnafu {
                        reason: "cannot copy from implementation row descriptor",
                    }
                    .fail();
                }
                DescriptorKind::Ipd => {
                    return crate::api::error::InconsistentDescriptorInfoSnafu {
                        reason: "cannot copy from implementation parameter descriptor",
                    }
                    .fail();
                }
            }
        }
        DescriptorAccess::Explicit { desc } => {
            let desc = desc.lock();
            CopyData::Ard(desc.clone_for_copy())
        }
    };

    let target_access = desc_from_handle(target_handle)?;

    match target_access {
        DescriptorAccess::Implicit { guard, kind } => {
            let mut inner = guard.inner.lock();
            match kind {
                DescriptorKind::Ard => match source_data {
                    CopyData::Ard(data) => {
                        data.apply_to(&mut inner.ard);
                        Ok(())
                    }
                    CopyData::Apd(data) => {
                        data.apply_to_ard(&mut inner.ard);
                        Ok(())
                    }
                },
                DescriptorKind::Apd => match source_data {
                    CopyData::Apd(data) => {
                        data.apply_to(&mut inner.apd);
                        Ok(())
                    }
                    CopyData::Ard(data) => {
                        data.apply_to_apd(&mut inner.apd);
                        Ok(())
                    }
                },
                DescriptorKind::Ird => crate::api::error::CannotModifyIrdSnafu.fail(),
                DescriptorKind::Ipd => crate::api::error::InconsistentDescriptorInfoSnafu {
                    reason: "cannot copy into implementation parameter descriptor",
                }
                .fail(),
            }
        }
        DescriptorAccess::Explicit { desc } => {
            let mut desc = desc.lock();
            match source_data {
                CopyData::Ard(data) => {
                    data.apply_to(&mut desc);
                    Ok(())
                }
                CopyData::Apd(data) => {
                    data.apply_to_ard(&mut desc);
                    Ok(())
                }
            }
        }
    }
}

enum CopyData {
    Ard(ArdCopyData),
    Apd(ApdCopyData),
}

struct ArdCopyData {
    bindings: std::collections::HashMap<u16, crate::conversion::Binding>,
    array_size: usize,
    bind_type: sql::ULen,
    bind_offset_ptr: *mut sql::Len,
}

impl ArdCopyData {
    fn apply_to(self, target: &mut crate::api::ArdDescriptor) {
        target.bindings = self.bindings;
        target.array_size = self.array_size;
        target.bind_type = self.bind_type;
        target.bind_offset_ptr = self.bind_offset_ptr;
    }

    fn apply_to_apd(self, target: &mut crate::api::ApdDescriptor) {
        target.records.clear();
        for (col, binding) in &self.bindings {
            target.records.insert(
                *col,
                crate::api::types::ApdRecord {
                    value_type: binding.target_type,
                    data_ptr: binding.target_value_ptr,
                    buffer_length: binding.buffer_length,
                    str_len_or_ind_ptr: binding.indicator_ptr,
                },
            );
        }
        target.array_size = self.array_size;
        target.bind_type = self.bind_type;
        target.bind_offset_ptr = self.bind_offset_ptr;
    }
}

impl crate::api::ArdDescriptor {
    fn clone_for_copy(&self) -> ArdCopyData {
        ArdCopyData {
            bindings: self.bindings.clone(),
            array_size: self.array_size,
            bind_type: self.bind_type,
            bind_offset_ptr: self.bind_offset_ptr,
        }
    }
}

struct ApdCopyData {
    records: std::collections::HashMap<u16, crate::api::types::ApdRecord>,
    array_size: usize,
    bind_type: sql::ULen,
    bind_offset_ptr: *mut sql::Len,
    array_status_ptr: *mut u16,
}

impl ApdCopyData {
    fn apply_to(self, target: &mut crate::api::ApdDescriptor) {
        target.records = self.records;
        target.array_size = self.array_size;
        target.bind_type = self.bind_type;
        target.bind_offset_ptr = self.bind_offset_ptr;
        target.array_status_ptr = self.array_status_ptr;
    }

    fn apply_to_ard(self, target: &mut crate::api::ArdDescriptor) {
        target.bindings.clear();
        for (col, record) in &self.records {
            target.bindings.insert(
                *col,
                crate::conversion::Binding {
                    target_type: record.value_type,
                    target_value_ptr: record.data_ptr,
                    buffer_length: record.buffer_length,
                    indicator_ptr: record.str_len_or_ind_ptr,
                    octet_length_ptr: record.str_len_or_ind_ptr,
                    ..Default::default()
                },
            );
        }
        target.array_size = self.array_size;
        target.bind_type = self.bind_type;
        target.bind_offset_ptr = self.bind_offset_ptr;
    }
}

impl crate::api::ApdDescriptor {
    fn clone_for_copy(&self) -> ApdCopyData {
        ApdCopyData {
            records: self.records.clone(),
            array_size: self.array_size,
            bind_type: self.bind_type,
            bind_offset_ptr: self.bind_offset_ptr,
            array_status_ptr: self.array_status_ptr,
        }
    }
}
