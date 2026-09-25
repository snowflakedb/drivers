use crate::DRIVER;
use crate::error::{BridgeError, ConnectionOperation, UnusableConnection};
use crate::session_params::KnownSessionParameters;
use sf_core::apis::database_driver_v1::{ApiError, ConnectionUsability};
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
        let _ = DRIVER.connection_release(self.connection);
        let _ = DRIVER.database_release(self.database);
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
    lifecycle_lock: Arc<Mutex<()>>,
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
            lifecycle_lock: Arc::new(Mutex::new(())),
        }
    }

    pub(crate) async fn connect(&self) -> Result<(), BridgeError> {
        let _lifecycle = self.lifecycle_lock.lock().await;
        match DRIVER.connection_is_usable(self.handles.connection).await {
            Ok(ConnectionUsability::Usable) => Err(BridgeError::AlreadyConnected),
            Ok(_) => self.init().await.map_err(BridgeError::from),
            Err(_) => Err(BridgeError::UnusableConnection(
                ConnectionOperation::Request,
                UnusableConnection::Terminated,
            )),
        }
    }

    pub(crate) async fn ready(&self) -> Result<Ready, BridgeError> {
        match self.unusable().await {
            Some(unusable) => Err(BridgeError::UnusableConnection(
                ConnectionOperation::Request,
                unusable,
            )),
            None => Ok(Ready(self.handles.clone())),
        }
    }

    pub(crate) async fn is_up(&self) -> bool {
        self.unusable().await.is_none()
    }

    pub(crate) async fn is_valid(&self) -> bool {
        match self.ready().await {
            Ok(ready) => DRIVER
                .connection_heartbeat(ready.connection())
                .await
                .unwrap_or(false),
            Err(_) => false,
        }
    }

    pub(crate) async fn known_session_parameters(
        &self,
    ) -> Result<KnownSessionParameters, BridgeError> {
        if self.unusable().await.is_some() {
            return Ok(KnownSessionParameters::defaults());
        }
        KnownSessionParameters::from_connection(self.handles.connection).await
    }

    pub(crate) async fn close(&self) -> Result<(), BridgeError> {
        let _lifecycle = self.lifecycle_lock.lock().await;
        if let Some(unusable) = self.unusable().await {
            return Err(BridgeError::UnusableConnection(
                ConnectionOperation::Destroy,
                unusable,
            ));
        }
        let close = DRIVER.connection_close(self.handles.connection).await;
        if close.is_ok() {
            self.handles.release();
        }
        close.map_err(BridgeError::from)
    }

    async fn init(&self) -> Result<(), ApiError> {
        let result = DRIVER
            .connection_init(None, self.handles.connection, self.handles.database)
            .await;
        if result.is_err() {
            let _ = DRIVER.connection_close(self.handles.connection).await;
        }
        result
    }

    async fn unusable(&self) -> Option<UnusableConnection> {
        match DRIVER.connection_is_usable(self.handles.connection).await {
            Ok(ConnectionUsability::Usable) => None,
            Ok(ConnectionUsability::NeverEstablished) => Some(UnusableConnection::NeverEstablished),
            Ok(ConnectionUsability::Terminated) | Err(_) => Some(UnusableConnection::Terminated),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new(DRIVER.connection_new(), DRIVER.database_new())
    }

    #[tokio::test]
    async fn a_connection_nobody_touched_was_never_established() {
        let session = session();

        assert!(matches!(
            session.unusable().await,
            Some(UnusableConnection::NeverEstablished)
        ));
    }

    #[tokio::test]
    async fn a_closed_connection_is_terminated() {
        let session = session();
        DRIVER
            .connection_close(session.handles.connection)
            .await
            .unwrap();

        assert!(matches!(
            session.unusable().await,
            Some(UnusableConnection::Terminated)
        ));
    }

    #[tokio::test]
    async fn a_released_handle_is_terminated() {
        let session = session();
        DRIVER
            .connection_release(session.handles.connection)
            .unwrap();

        assert!(matches!(
            session.unusable().await,
            Some(UnusableConnection::Terminated)
        ));
    }

    #[tokio::test]
    async fn connecting_after_the_handle_is_gone_does_not_call_init() {
        let session = session();
        DRIVER
            .connection_release(session.handles.connection)
            .unwrap();

        assert!(matches!(
            session.connect().await,
            Err(BridgeError::UnusableConnection(
                ConnectionOperation::Request,
                UnusableConnection::Terminated,
            ))
        ));
    }

    #[tokio::test]
    async fn an_unusable_session_answers_session_parameters_with_client_defaults() {
        let never_established = session();
        let released = session();
        DRIVER
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
}
