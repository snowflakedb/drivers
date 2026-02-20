use crate::api::{DescField, DescriptorRef, OdbcResult, desc_ref_from_handle};
use crate::cdata_types::CDataType;
use odbc_sys as sql;
use tracing;

/// Get a descriptor field value
pub fn get_desc_field(
    desc_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    _buffer_length: sql::Integer,
    _string_length_ptr: *mut sql::Integer,
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

    let field = DescField::try_from(field_identifier)?;
    let desc_ref = desc_ref_from_handle(desc_handle)?;

    match desc_ref {
        DescriptorRef::Ard(desc) => get_ard_field(desc, rec_number, field, value_ptr),
        DescriptorRef::Ird(desc) => get_ird_field(desc, rec_number, field, value_ptr),
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
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field as i32,
                }
                .fail()
            }
        }
    } else {
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
            DescField::Type | DescField::ConciseType => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut sql::SmallInt,
                        binding.target_type as sql::SmallInt,
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
            _ => {
                tracing::warn!("get_desc_field: unsupported ARD record field {:?}", field);
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field as i32,
                }
                .fail()
            }
        }
    }
}

fn get_ird_field(
    desc: &crate::api::IrdDescriptor,
    rec_number: sql::SmallInt,
    field: DescField,
    value_ptr: sql::Pointer,
) -> OdbcResult<()> {
    if rec_number == 0 {
        match field {
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
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field as i32,
                }
                .fail()
            }
        }
    } else {
        tracing::warn!(
            "get_desc_field: IRD record fields not supported (rec={})",
            rec_number
        );
        crate::api::error::NoMoreDataSnafu.fail()
    }
}

/// Set a descriptor field value
pub fn set_desc_field(
    desc_handle: sql::Handle,
    rec_number: sql::SmallInt,
    field_identifier: sql::SmallInt,
    value_ptr: sql::Pointer,
    _buffer_length: sql::Integer,
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
    let desc_ref = desc_ref_from_handle(desc_handle)?;

    match desc_ref {
        DescriptorRef::Ard(desc) => set_ard_field(desc, rec_number, field, value_ptr),
        DescriptorRef::Ird(desc) => set_ird_field(desc, rec_number, field, value_ptr),
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
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field as i32,
                }
                .fail()
            }
        }
    } else {
        let column_number = rec_number as u16;

        match field {
            DescField::Type | DescField::ConciseType => {
                let raw = value_ptr as i16;
                let c_type = CDataType::try_from(raw).map_err(|unknown| {
                    tracing::error!("set_desc_field: unknown C data type discriminant {unknown}");
                    crate::api::error::OdbcError::InvalidApplicationBufferType {
                        location: snafu::location!(),
                    }
                })?;
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
            _ => {
                tracing::warn!("set_desc_field: unsupported ARD record field {:?}", field);
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field as i32,
                }
                .fail()
            }
        }
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
                tracing::warn!("set_desc_field: unsupported IRD header field {:?}", field);
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field as i32,
                }
                .fail()
            }
        }
    } else {
        tracing::warn!(
            "set_desc_field: IRD record fields are read-only (rec={})",
            rec_number
        );
        crate::api::error::UnknownAttributeSnafu {
            attribute: field as i32,
        }
        .fail()
    }
}
