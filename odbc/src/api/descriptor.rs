use crate::api::{DescField, OdbcResult, desc_from_handle};
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
    let desc = desc_from_handle(desc_handle)?;

    if rec_number == 0 {
        // Header fields (record 0)
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
            _ => {
                tracing::warn!("get_desc_field: unsupported header field {:?}", field);
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field_identifier as i32,
                }
                .fail()
            }
        }
    } else {
        // Record-level fields
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
            DescField::IndicatorPtr | DescField::OctetLengthPtr => {
                unsafe {
                    std::ptr::write_unaligned(
                        value_ptr as *mut *mut sql::Len,
                        binding.str_len_or_ind_ptr,
                    );
                }
                Ok(())
            }
            _ => {
                tracing::warn!("get_desc_field: unsupported record field {:?}", field);
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field_identifier as i32,
                }
                .fail()
            }
        }
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
    let desc = desc_from_handle(desc_handle)?;

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
            _ => {
                tracing::warn!("set_desc_field: unsupported header field {:?}", field);
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field_identifier as i32,
                }
                .fail()
            }
        }
    } else {
        // Record-level set fields (for SQL_C_NUMERIC support etc.)
        let column_number = rec_number as u16;

        match field {
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
                if let Some(binding) = desc.bindings.get_mut(&column_number) {
                    binding.precision = Some(precision);
                } else {
                    tracing::warn!(
                        "set_desc_field: no binding for record {column_number}, ignoring precision"
                    );
                }
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
                if let Some(binding) = desc.bindings.get_mut(&column_number) {
                    binding.scale = Some(scale);
                } else {
                    tracing::warn!(
                        "set_desc_field: no binding for record {column_number}, ignoring scale"
                    );
                }
                Ok(())
            }
            DescField::DataPtr => {
                tracing::debug!("set_desc_field: setting data_ptr on record {column_number}");
                if let Some(binding) = desc.bindings.get_mut(&column_number) {
                    binding.target_value_ptr = value_ptr;
                } else {
                    tracing::warn!(
                        "set_desc_field: no binding for record {column_number}, ignoring data_ptr"
                    );
                }
                Ok(())
            }
            _ => {
                tracing::warn!("set_desc_field: unsupported record field {:?}", field);
                crate::api::error::UnknownAttributeSnafu {
                    attribute: field_identifier as i32,
                }
                .fail()
            }
        }
    }
}
