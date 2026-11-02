use crate::BRIDGE;
use crate::error::BridgeError;
use napi::tokio::sync::OnceCell;
use sf_core::config::param_names;
use sf_core::config::settings::Setting;
use sf_core::handle_manager::Handle;

struct TelemetryIdentity {
    driver_type: String,
    driver_version: String,
    source: String,
}

impl TelemetryIdentity {
    async fn load(connection_handle: Handle) -> Result<Self, BridgeError> {
        let identity = BRIDGE
            .driver
            .get_wrapper_identity(connection_handle)
            .await?
            .unwrap_or_default();
        let application = BRIDGE
            .driver
            .connection_get_parameter(
                connection_handle,
                param_names::APPLICATION.as_str().to_string(),
            )
            .await?;
        let source = resolve_source(application, &identity.driver_name);
        Ok(Self {
            driver_type: identity.driver_name,
            driver_version: identity.driver_version,
            source,
        })
    }
}

fn resolve_source(application: Option<Setting>, driver_name: &str) -> String {
    match application {
        Some(Setting::String(application)) if !application.trim().is_empty() => application,
        _ => driver_name.to_string(),
    }
}

#[derive(Default)]
pub(crate) struct Telemetry {
    identity: OnceCell<TelemetryIdentity>,
}

impl Telemetry {
    pub(crate) async fn send_event(
        &self,
        connection_handle: Handle,
        event_type: String,
        query_id: Option<String>,
    ) -> Result<(), BridgeError> {
        let identity = self
            .identity
            .get_or_try_init(|| TelemetryIdentity::load(connection_handle))
            .await?;
        let message = serde_json::json!({
            "driver_type": identity.driver_type,
            "driver_version": identity.driver_version,
            "source": identity.source,
            "type": event_type,
            "value": { "queryId": query_id },
        });
        BRIDGE
            .driver
            .telemetry_send_log(
                connection_handle,
                message.to_string(),
                chrono::Utc::now().timestamp_millis(),
            )
            .await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_is_the_application_when_set() {
        assert_eq!(
            resolve_source(Some(Setting::String("MY_APPLICATION".into())), "JavaScript"),
            "MY_APPLICATION"
        );
    }

    #[test]
    fn source_falls_back_to_the_driver_name_when_application_is_unset_or_blank() {
        for application in [
            None,
            Some(Setting::String(String::new())),
            Some(Setting::String("   ".into())),
        ] {
            assert_eq!(resolve_source(application, "JavaScript"), "JavaScript");
        }
    }
}
