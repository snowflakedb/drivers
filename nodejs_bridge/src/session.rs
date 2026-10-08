use crate::BRIDGE;
use crate::error::{BridgeError, ConnectionOperation, UnusableConnection};
use crate::session_params::KnownSessionParameters;
use sf_core::apis::database_driver_v1::{ApiError, ConnectionInfo, ConnectionUsability};
use sf_core::handle_manager::Handle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

struct Handles {
    connection: Handle,
    database: Handle,
    released: AtomicBool,
}

impl Handles {
    fn release(&self) {
        if self.released.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = BRIDGE.driver.connection_release(self.connection);
        let _ = BRIDGE.driver.database_release(self.database);
    }
}

impl Drop for Handles {
    fn drop(&mut self) {
        self.release();
    }
}

#[derive(Clone)]
pub(crate) struct Session {
    handles: Arc<Handles>,
    connect_lock: Arc<Mutex<()>>,
}

pub(crate) struct Ready(Arc<Handles>);

impl Ready {
    pub(crate) fn connection(&self) -> Handle {
        self.0.connection
    }
}

impl Session {
    pub(crate) fn new(connection: Handle, database: Handle) -> Self {
        Self {
            handles: Arc::new(Handles {
                connection,
                database,
                released: AtomicBool::new(false),
            }),
            connect_lock: Arc::new(Mutex::new(())),
        }
    }

    pub(crate) async fn connect(&self) -> Result<(), BridgeError> {
        let Ok(_connecting) = self.connect_lock.try_lock() else {
            return Err(BridgeError::AlreadyConnecting);
        };
        match BRIDGE
            .driver
            .connection_is_usable(self.handles.connection)
            .await
        {
            Ok(ConnectionUsability::Usable) => return Err(BridgeError::AlreadyConnected),
            Ok(ConnectionUsability::Terminated) | Err(_) => {
                return Err(BridgeError::ConnectionTerminated);
            }
            Ok(ConnectionUsability::NeverEstablished) => {}
        }
        let result = BRIDGE
            .driver
            .connection_init(None, self.handles.connection, self.handles.database)
            .await;
        if result.is_err() {
            let _ = BRIDGE
                .driver
                .connection_close(self.handles.connection)
                .await;
        }
        result.map_err(BridgeError::from)
    }

    pub(crate) async fn ready(&self) -> Result<Ready, BridgeError> {
        match self.unusable_reason().await {
            Some(reason) => Err(BridgeError::UnusableConnection(
                ConnectionOperation::Request,
                reason,
            )),
            None => Ok(Ready(self.handles.clone())),
        }
    }

    pub(crate) async fn is_up(&self) -> bool {
        self.unusable_reason().await.is_none()
    }

    pub(crate) async fn is_valid(&self) -> bool {
        match self.ready().await {
            Ok(ready) => BRIDGE
                .driver
                .connection_heartbeat(ready.connection())
                .await
                .unwrap_or(false),
            Err(_) => false,
        }
    }

    pub(crate) async fn known_session_parameters(
        &self,
    ) -> Result<KnownSessionParameters, BridgeError> {
        if self.unusable_reason().await.is_some() {
            return Ok(KnownSessionParameters::defaults());
        }
        KnownSessionParameters::from_connection(self.handles.connection).await
    }

    pub(crate) async fn token_info(&self) -> Result<Option<ConnectionInfo>, ApiError> {
        if self.unusable_reason().await.is_some() {
            return Ok(None);
        }
        BRIDGE
            .driver
            .connection_get_info(self.handles.connection)
            .await
            .map(Some)
    }

    pub(crate) async fn close(&self) -> Result<(), BridgeError> {
        let _connecting = self.connect_lock.lock().await;
        if let Some(reason) = self.unusable_reason().await {
            return Err(BridgeError::UnusableConnection(
                ConnectionOperation::Destroy,
                reason,
            ));
        }
        let close = BRIDGE
            .driver
            .connection_close(self.handles.connection)
            .await;
        if close.is_ok() {
            self.handles.release();
        }
        close.map_err(BridgeError::from)
    }

    async fn unusable_reason(&self) -> Option<UnusableConnection> {
        match BRIDGE
            .driver
            .connection_is_usable(self.handles.connection)
            .await
        {
            Ok(ConnectionUsability::Usable) => None,
            Ok(ConnectionUsability::NeverEstablished) => Some(UnusableConnection::NeverEstablished),
            Ok(ConnectionUsability::Terminated) | Err(_) => Some(UnusableConnection::Terminated),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_core::config::param_names;
    use sf_core::config::settings::Setting;
    use std::collections::HashMap;

    fn session() -> Session {
        Session::new(BRIDGE.driver.connection_new(), BRIDGE.driver.database_new())
    }

    async fn session_that_adopted_tokens() -> Session {
        let connection = BRIDGE.driver.connection_new();
        let database = BRIDGE.driver.database_new();
        BRIDGE.driver.database_init(database).unwrap();
        let options = HashMap::from([
            (
                param_names::ACCOUNT.as_str().to_string(),
                Setting::String("adopting-account".to_string()),
            ),
            (
                param_names::SESSION_TOKEN.as_str().to_string(),
                Setting::String("handed-over-session".to_string()),
            ),
            (
                param_names::MASTER_TOKEN.as_str().to_string(),
                Setting::String("handed-over-master".to_string()),
            ),
        ]);
        BRIDGE
            .driver
            .connection_set_options(connection, options, false, None)
            .await
            .unwrap();
        BRIDGE
            .driver
            .connection_init(None, connection, database)
            .await
            .unwrap();
        Session::new(connection, database)
    }

    async fn is_terminated(session: &Session) -> bool {
        matches!(
            session.unusable_reason().await,
            Some(UnusableConnection::Terminated)
        )
    }

    #[tokio::test]
    async fn a_connection_nobody_touched_was_never_established() {
        let session = session();

        assert!(matches!(
            session.unusable_reason().await,
            Some(UnusableConnection::NeverEstablished)
        ));
    }

    #[tokio::test]
    async fn a_closed_connection_is_terminated() {
        let session = session();
        BRIDGE
            .driver
            .connection_close(session.handles.connection)
            .await
            .unwrap();

        assert!(is_terminated(&session).await);
    }

    #[tokio::test]
    async fn a_released_handle_is_terminated() {
        let session = session();
        BRIDGE
            .driver
            .connection_release(session.handles.connection)
            .unwrap();

        assert!(is_terminated(&session).await);
    }

    #[tokio::test]
    async fn a_second_connect_while_one_is_in_flight_is_refused() {
        let session = session();
        let _connecting = session.connect_lock.lock().await;

        assert!(matches!(
            session.connect().await,
            Err(BridgeError::AlreadyConnecting)
        ));
    }

    #[tokio::test]
    async fn connecting_a_closed_connection_is_refused() {
        let session = session();
        BRIDGE
            .driver
            .connection_close(session.handles.connection)
            .await
            .unwrap();

        assert!(matches!(
            session.connect().await,
            Err(BridgeError::ConnectionTerminated)
        ));
    }

    #[tokio::test]
    async fn connecting_after_the_handle_is_gone_does_not_log_in() {
        let session = session();
        BRIDGE
            .driver
            .connection_release(session.handles.connection)
            .unwrap();

        assert!(matches!(
            session.connect().await,
            Err(BridgeError::ConnectionTerminated)
        ));
    }

    #[tokio::test]
    async fn an_unusable_session_answers_session_parameters_with_client_defaults() {
        let never_established = session();
        let released = session();
        BRIDGE
            .driver
            .connection_release(released.handles.connection)
            .unwrap();

        let Ok(never_established) = never_established.known_session_parameters().await else {
            panic!("defaults, not a core error");
        };
        let Ok(released) = released.known_session_parameters().await else {
            panic!("defaults, not a core error");
        };

        for params in [never_established, released] {
            assert_eq!(params.time_output_format, "HH24:MI:SS");
            assert!(!params.js_treat_integer_as_big_int);
            assert_eq!(params.client_stage_array_binding_threshold, 100_000);
        }
    }

    #[tokio::test]
    async fn an_unusable_session_answers_token_info_with_none() {
        let never_established = session();
        let released = session();
        BRIDGE
            .driver
            .connection_release(released.handles.connection)
            .unwrap();

        let Ok(never_established) = never_established.token_info().await else {
            panic!("no token info, not a core error");
        };
        let Ok(released) = released.token_info().await else {
            panic!("no token info, not a core error");
        };

        assert!(never_established.is_none());
        assert!(released.is_none());
    }

    #[tokio::test]
    async fn a_session_that_adopted_tokens_is_up_and_refuses_connect() {
        let session = session_that_adopted_tokens().await;

        assert!(session.is_up().await);
        assert!(matches!(
            session.connect().await,
            Err(BridgeError::AlreadyConnected)
        ));
    }
}
