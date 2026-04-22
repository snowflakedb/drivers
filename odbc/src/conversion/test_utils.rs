#[cfg(test)]
pub(crate) mod helpers {
    use crate::api::CDataType;
    use crate::conversion::traits::Binding;
    use odbc_sys as sql;

    pub fn binding_for_value<T>(
        target_type: CDataType,
        value: &mut T,
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: value as *mut T as sql::Pointer,
            buffer_length: 0,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
        }
    }

    pub fn binding_for_char_buffer(
        target_type: CDataType,
        buffer: &mut [u8],
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: buffer.len() as sql::Len,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
        }
    }

    pub fn binding_for_wchar_buffer(buffer: &mut [u16], str_len: &mut sql::Len) -> Binding {
        Binding {
            target_type: CDataType::WChar,
            target_value_ptr: buffer.as_mut_ptr() as sql::Pointer,
            buffer_length: (buffer.len() * 2) as sql::Len,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
        }
    }

    pub fn zero_interval() -> sql::IntervalStruct {
        sql::IntervalStruct {
            interval_type: 0,
            interval_sign: 0,
            interval_value: sql::IntervalUnion {
                day_second: sql::DaySecond::default(),
            },
        }
    }

    pub fn binding_for_interval(
        target_type: CDataType,
        value: &mut sql::IntervalStruct,
        str_len: &mut sql::Len,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: value as *mut sql::IntervalStruct as sql::Pointer,
            buffer_length: 0,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            ..Default::default()
        }
    }

    pub fn binding_for_interval_with_precision(
        target_type: CDataType,
        value: &mut sql::IntervalStruct,
        str_len: &mut sql::Len,
        precision: i16,
    ) -> Binding {
        Binding {
            target_type,
            target_value_ptr: value as *mut sql::IntervalStruct as sql::Pointer,
            buffer_length: 0,
            octet_length_ptr: str_len as *mut sql::Len,
            indicator_ptr: str_len as *mut sql::Len,
            datetime_interval_precision: Some(precision),
            ..Default::default()
        }
    }
}
