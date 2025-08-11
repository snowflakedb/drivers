use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use arrow::{
    array::{Int64Array, RecordBatch},
    datatypes::DataType,
};

use crate::rest::snowflake::query_request;

use super::connection::Connection;
use super::settings::Setting;

#[derive(Debug, Clone)]
pub enum StatementError {
    UnsupportedBindParameterType(String),
    InvalidStateTransition { from: StatementState, msg: String },
}

fn parameters_from_record_batch(
    record_batch: &RecordBatch,
) -> Result<HashMap<String, query_request::BindParameter>, StatementError> {
    let mut parameters = HashMap::new();
    for i in 0..record_batch.num_columns() {
        let column = record_batch.column(i);
        match column.data_type() {
            DataType::Int64 => {
                let value = column
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .value(0);
                let json_value = serde_json::Value::String(value.to_string());
                parameters.insert(
                    format!("{}", i + 1),
                    query_request::BindParameter {
                        type_: "FIXED".to_string(),
                        value: json_value,
                        format: None,
                        schema: None,
                    },
                );
            }
            _ => {
                return Err(StatementError::UnsupportedBindParameterType(
                    column.data_type().to_string(),
                ));
            }
        }
    }
    Ok(parameters)
}

pub struct Statement {
    pub state: StatementState,
    pub settings: HashMap<String, Setting>,
    pub query: Option<String>,
    pub parameter_bindings: Option<RecordBatch>,
    pub conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub enum StatementState {
    Initialized,
    Executed,
}

impl Statement {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Statement {
            settings: HashMap::new(),
            state: StatementState::Initialized,
            query: None,
            parameter_bindings: None,
            conn,
        }
    }

    pub fn bind_parameters(&mut self, record_batch: RecordBatch) -> Result<(), StatementError> {
        match self.state {
            StatementState::Initialized => {
                self.parameter_bindings = Some(record_batch);
            }
            _ => {
                return Err(StatementError::InvalidStateTransition {
                    from: self.state.clone(),
                    msg: format!("Cannot bind parameters in state={:?}", self.state),
                });
            }
        }
        Ok(())
    }

    pub fn get_query_parameter_bindings(
        &self,
    ) -> Result<Option<HashMap<String, query_request::BindParameter>>, StatementError> {
        match self.parameter_bindings.as_ref() {
            Some(parameters) => Ok(Some(parameters_from_record_batch(parameters)?)),
            None => Ok(None),
        }
    }
}
