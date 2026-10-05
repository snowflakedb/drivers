use crate::DRIVER;
use crate::error::{BridgeError, ConnectionOperation, UnusableConnection};
use crate::session_params::KnownSessionParameters;
use sf_core::apis::database_driver_v1::{ApiError, ConnectionInfo, ConnectionUsability};
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
    deferred_init: Arc<AtomicBool>,
}

pub(crate) struct Ready(Arc<Handles>);

impl Ready {
    pub(crate) fn connection(&self) -> Handle {
        self.0.connection
    }
}

impl Session {
    pub(crate) fn new(connection: Handle, database: Handle, deferred_init: bool) -> Self {
        Self {
            handles: Arc::new(Handles {
                connection,
                database,
                released: AtomicBool::new(false),
            }),
            login_in_flight: Arc::new(Mutex::new(false)),
            login_finished: Arc::new(Notify::new()),
            deferred_init: Arc::new(AtomicBool::new(deferred_init)),
        }
    }

    pub(crate) async fn connect(&self) -> Result<(), BridgeError> {
        if self.deferred_init.load(Ordering::Acquire) {
            return Err(BridgeError::AlreadyConnected);
        }
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
        self.settle().await?;
        match self.unusable().await {
            Some(unusable) => Err(BridgeError::UnusableConnection(
                ConnectionOperation::Request,
                unusable,
            )),
            None => Ok(Ready(self.handles.clone())),
        }
    }

    pub(crate) async fn is_up(&self) -> bool {
        if self.deferred_init.load(Ordering::Acquire) {
            return !matches!(self.unusable().await, Some(UnusableConnection::Terminated));
        }
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
        self.settle().await?;
        if self.unusable().await.is_some() {
            return Ok(KnownSessionParameters::defaults());
        }
        KnownSessionParameters::from_connection(self.handles.connection).await
    }

    pub(crate) async fn info(&self) -> Result<Option<ConnectionInfo>, ApiError> {
        if self.unusable().await.is_some() {
            return Ok(None);
        }
        DRIVER
            .connection_get_info(self.handles.connection)
            .await
            .map(Some)
    }

    pub(crate) async fn close(&self) -> Result<(), BridgeError> {
        self.settle().await?;
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

    async fn settle(&self) -> Result<(), BridgeError> {
        if !self.deferred_init.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut login_in_flight = self.wait_for_login_to_finish().await;
        if !self.deferred_init.load(Ordering::Acquire) {
            return Ok(());
        }
        *login_in_flight = true;
        drop(login_in_flight);
        let result = self.init().await.map_err(BridgeError::from);
        self.deferred_init.store(false, Ordering::Release);
        *self.login_in_flight.lock().await = false;
        self.login_finished.notify_waiters();
        result
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
        Session::new(DRIVER.connection_new(), DRIVER.database_new(), false)
    }

    fn deferred_session() -> Session {
        Session::new(DRIVER.connection_new(), DRIVER.database_new(), true)
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

    #[tokio::test]
    async fn an_unusable_session_answers_token_info_with_none() {
        let never_established = session();
        let released = session();
        DRIVER
            .connection_release(released.handles.connection)
            .unwrap();

        let Ok(never_established) = never_established.info().await else {
            panic!("empty token info, not a core error");
        };
        let Ok(released) = released.info().await else {
            panic!("empty token info, not a core error");
        };

        assert!(never_established.is_none());
        assert!(released.is_none());
    }

    #[tokio::test]
    async fn a_deferred_session_is_up_before_init() {
        let session = deferred_session();

        assert!(session.is_up().await);
        assert!(matches!(
            session.unusable().await,
            Some(UnusableConnection::NeverEstablished)
        ));
    }

    #[tokio::test]
    async fn connecting_a_deferred_session_is_refused() {
        let session = deferred_session();

        assert!(matches!(
            session.connect().await,
            Err(BridgeError::AlreadyConnected)
        ));
        assert!(session.is_up().await);
    }

    #[tokio::test]
    async fn settle_waits_for_an_init_already_in_flight_instead_of_starting_another() {
        let session = deferred_session();
        *session.login_in_flight.lock().await = true;
        let finish_other_init = async {
            tokio::task::yield_now().await;
            session.deferred_init.store(false, Ordering::Release);
            *session.login_in_flight.lock().await = false;
            session.login_finished.notify_waiters();
        };

        let (settled, ()) = tokio::join!(session.settle(), finish_other_init);

        settled.unwrap();
        assert!(matches!(
            session.unusable().await,
            Some(UnusableConnection::NeverEstablished)
        ));
        assert!(!*session.login_in_flight.lock().await);
    }

    #[tokio::test]
    async fn a_failed_deferred_init_leaves_every_concurrent_request_with_an_error() {
        let session = deferred_session();

        let (first, second) = tokio::join!(session.ready(), session.ready());

        assert!(matches!(first, Err(BridgeError::Core(_))));
        assert!(matches!(
            second,
            Err(BridgeError::UnusableConnection(
                ConnectionOperation::Request,
                UnusableConnection::Terminated
            ))
        ));
        assert!(!session.is_up().await);
        assert!(matches!(
            session.connect().await,
            Err(BridgeError::ConnectionTerminated)
        ));
    }

    #[tokio::test]
    async fn settle_on_an_ordinary_session_does_not_initialize() {
        let session = session();

        session.settle().await.unwrap();

        assert!(matches!(
            session.unusable().await,
            Some(UnusableConnection::NeverEstablished)
        ));
        assert!(!session.is_up().await);
    }
}
