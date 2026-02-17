use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Request<'a> {
    #[serde(rename = "sqlText")]
    pub sql_text: String,
    #[serde(rename = "asyncExec")]
    pub async_exec: bool,
    #[serde(rename = "sequenceId")]
    pub sequence_id: u64,
    #[serde(rename = "querySubmissionTime")]
    pub query_submission_time: i64,
    #[serde(rename = "isInternal")]
    pub is_internal: bool,
    #[serde(rename = "describeOnly", skip_serializing_if = "Option::is_none")]
    pub describe_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    /// Raw JSON pass-through for parameter bindings.
    ///
    /// Points directly into the caller's memory -- zero allocation, zero copy.
    /// During HTTP serialization, serde writes the raw JSON string verbatim --
    /// no `Value` tree is ever built.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bindings: Option<&'a RawValue>,
    #[serde(rename = "bindStage", skip_serializing_if = "Option::is_none")]
    pub bind_stage: Option<String>,
    #[serde(rename = "queryContextDTO")]
    pub query_context: QueryContext,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BindParameter {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: serde_json::Value,
    #[serde(rename = "fmt", skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<BindingSchema>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BindingSchema {}

#[derive(Serialize)]
pub struct QueryContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<QueryContextEntry>>,
}

#[derive(Serialize)]
pub struct QueryContextEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextData>,
    pub id: i32,
    pub priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

#[derive(Serialize)]
pub struct ContextData {
    #[serde(rename = "base64Data", skip_serializing_if = "Option::is_none")]
    pub base64_data: Option<String>,
}
