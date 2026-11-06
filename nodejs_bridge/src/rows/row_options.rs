use napi_derive::napi;
use std::borrow::Cow;

use super::js_cell::JsCell;

#[napi(string_enum)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchAsStringType {
    String,
    Number,
    Boolean,
    Date,
    #[napi(value = "JSON")]
    Json,
    Buffer,
}

#[napi(object)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RowOptions {
    pub fetch_as_string: Vec<FetchAsStringType>,
    pub represent_null_as_string_null: bool,
}

impl RowOptions {
    pub(super) fn output_for(&self, token: Option<FetchAsStringType>) -> ColumnOutput {
        if token.is_some_and(|token| self.fetch_as_string.contains(&token)) {
            ColumnOutput::String {
                null_as_string: self.represent_null_as_string_null,
            }
        } else {
            ColumnOutput::Native
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColumnOutput {
    Native,
    String { null_as_string: bool },
}

impl ColumnOutput {
    pub(super) fn null_cell(self) -> JsCell<'static> {
        match self {
            Self::String {
                null_as_string: true,
            } => JsCell::Str(Cow::Borrowed("NULL")),
            _ => JsCell::Null,
        }
    }
}
