use crate::DRIVER;
use crate::error::{BridgeError, ConnectionOperation, UnusableConnection};
use crate::session_params::KnownSessionParameters;
use sf_core::apis::database_driver_v1::{ApiError, ConnectionUsability};
use sf_core::handle_manager::Handle;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, Notify};

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
    login_in_flight: Arc<Mutex<bool>>,
    login_finished: Arc<Notify>,
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
            login_in_flight: Arc::new(Mutex::new(false)),
            login_finished: Arc::new(Notify::new()),
        }
    }

    pub(crate) async fn connect(&self) -> Result<(), BridgeError> {
        {
            let mut login_in_flight = self.login_in_flight.lock().await;
            if *login_in_flight {
                return Err(BridgeError::AlreadyConnecting);
            }
            match DRIVER.connection_is_usable(self.handles.connection).await {
                Ok(ConnectionUsability::Usable) => return Err(BridgeError::AlreadyConnected),
                Ok(ConnectionUsability::Terminated) | Err(_) => {
                    return Err(BridgeError::ConnectionTerminated);
                }
                Ok(ConnectionUsability::NeverEstablished) => *login_in_flight = true,
            }
        }
        let result = self.init().await.map_err(BridgeError::from);
        *self.login_in_flight.lock().await = false;
        self.login_finished.notify_waiters();
        result
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
        let _login = self.wait_for_login_to_finish().await;
        self.close_connection().await
    }

    async fn wait_for_login_to_finish(&self) -> tokio::sync::MutexGuard<'_, bool> {
        loop {
            let mut finished = pin!(self.login_finished.notified());
            finished.as_mut().enable();
            let login_in_flight = self.login_in_flight.lock().await;
            if !*login_in_flight {
                return login_in_flight;
            }
            drop(login_in_flight);
            finished.await;
        }
    }

    async fn close_connection(&self) -> Result<(), BridgeError> {
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
    async fn a_second_connect_while_login_is_in_flight_is_refused() {
        let session = session();
        *session.login_in_flight.lock().await = true;

        assert!(matches!(
            session.connect().await,
            Err(BridgeError::AlreadyConnecting)
        ));
    }

    #[tokio::test]
    async fn connecting_a_closed_connection_is_refused() {
        let session = session();
        DRIVER
            .connection_close(session.handles.connection)
            .await
            .unwrap();

        assert!(matches!(
            session.connect().await,
            Err(BridgeError::ConnectionTerminated)
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
            Err(BridgeError::ConnectionTerminated)
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
