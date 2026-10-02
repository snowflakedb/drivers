use napi_derive::napi;
use sf_core::apis::database_driver_v1::ResultSetDescriptor;
use sf_core::apis::database_driver_v1::final_session_names::FinalSessionNames;

#[napi]
pub struct SessionState {
    names: FinalSessionNames,
}

impl SessionState {
    pub(crate) fn from_descriptor(descriptor: &ResultSetDescriptor) -> Self {
        Self {
            names: descriptor.final_session_names.clone(),
        }
    }
}

#[napi]
impl SessionState {
    #[napi]
    pub fn get_current_role(&self) -> Option<String> {
        self.names.role.clone()
    }

    #[napi]
    pub fn get_current_warehouse(&self) -> Option<String> {
        self.names.warehouse.clone()
    }

    #[napi]
    pub fn get_current_database(&self) -> Option<String> {
        self.names.database.clone()
    }

    #[napi]
    pub fn get_current_schema(&self) -> Option<String> {
        self.names.schema.clone()
    }
}
