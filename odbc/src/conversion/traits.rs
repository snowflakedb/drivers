use odbc_sys as sql;

use crate::cdata_types::CDataType;
use crate::conversion::error::{ReadArrowError, WriteOdbcError};
use crate::conversion::warning::{Warning, Warnings};

#[derive(Debug, Default)]
pub struct Binding {
    pub target_type: CDataType,
    pub target_value_ptr: sql::Pointer,
    pub buffer_length: sql::Len,
    pub str_len_or_ind_ptr: *mut sql::Len,
}

impl Binding {
    pub fn write_fixed<T>(&self, value: T) {
        unsafe {
            if !self.target_value_ptr.is_null() {
                std::ptr::write(self.target_value_ptr as *mut T, value);
            }
            if !self.str_len_or_ind_ptr.is_null() {
                std::ptr::write(
                    self.str_len_or_ind_ptr,
                    std::mem::size_of::<T>() as sql::Len,
                );
            }
        }
    }

    pub fn write_char_string(&self, src: &str) -> Warnings {
        let max_len = self.buffer_length as usize;
        let mut dst_idx = 0;
        let value_ptr = self.target_value_ptr as *mut u8;
        for c in src.chars() {
            if dst_idx == max_len - 1 {
                unsafe {
                    std::ptr::write(value_ptr.add(max_len - 1), 0);
                    if !self.str_len_or_ind_ptr.is_null() {
                        std::ptr::write(self.str_len_or_ind_ptr, sql::NO_TOTAL);
                    }
                }
                return vec![Warning::StringDataTruncated];
            }
            let byte = if c.is_ascii() { c as u8 } else { 0x1a };
            unsafe {
                std::ptr::write(value_ptr.add(dst_idx), byte);
            }
            dst_idx += 1;
        }
        unsafe {
            std::ptr::write(value_ptr.add(dst_idx), 0);
            if !self.str_len_or_ind_ptr.is_null() {
                std::ptr::write(self.str_len_or_ind_ptr, dst_idx as sql::Len);
            }
        }
        vec![]
    }

    pub fn write_wchar_string(&self, src: &str) -> Warnings {
        let max_len = (self.buffer_length / 2) as usize;
        let value_ptr = self.target_value_ptr as *mut u16;
        let mut dst_idx = 0;
        for c in src.encode_utf16() {
            if dst_idx == max_len - 1 {
                unsafe {
                    std::ptr::write(value_ptr.add(max_len - 1), 0);
                    if !self.str_len_or_ind_ptr.is_null() {
                        std::ptr::write(self.str_len_or_ind_ptr, sql::NO_TOTAL);
                    }
                }
                return vec![Warning::StringDataTruncated];
            }
            unsafe {
                std::ptr::write(value_ptr.add(dst_idx), c);
            }
            dst_idx += 1;
        }
        unsafe {
            std::ptr::write(value_ptr.add(dst_idx), 0);
            if !self.str_len_or_ind_ptr.is_null() {
                // COMPATIBILITY: ODBC 3.80 specification says that the string length should be the number of characters, not the number of bytes.
                // However, older versions of Snowflake ODBC driver returns the number of bytes.
                // So we need to convert the number of characters to the number of bytes.
                let num_characters = dst_idx as sql::Len;
                let num_bytes = num_characters * 2;
                std::ptr::write(self.str_len_or_ind_ptr, num_bytes);
            }
        }
        vec![]
    }
}

pub trait WriteODBCType: SnowflakeType {
    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
    ) -> Result<Warnings, WriteOdbcError>;
}

pub trait SnowflakeType {
    type Representation<'a>: Sized;
}

pub trait ReadArrowType<ArrowArrayType>: SnowflakeType {
    #[allow(clippy::wrong_self_convention)]
    fn read_arrow_type<'a>(
        &self,
        array: &'a ArrowArrayType,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError>;
}
