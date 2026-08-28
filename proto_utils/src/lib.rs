#[derive(Debug)]
pub enum ProtoError<T> {
    Transport(String),
    Application(T),
}

pub trait Transport {
    fn handle_message(
        &self,
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> impl Future<Output = Result<Vec<u8>, ProtoError<Vec<u8>>>> + Send;
}

/// A [`Transport`] that can dispatch an operation under a cancellation handle,
/// letting any thread cancel it afterwards.
///
/// Separate from `Transport` so an implementation that cannot cancel is still a
/// valid transport, and so that dispatching *without* a handle stays the plain,
/// obviously-uncancellable call rather than one that silently ignores an
/// operation id.
///
/// Deliberately expressed in opaque `u64` handles, with no cancellation
/// primitive in the signatures: this crate has no dependencies and must keep
/// none. The implementation owns whatever it keys those handles to.
pub trait CancellableTransport: Transport {
    /// Mint a handle for an operation that has not been dispatched yet.
    ///
    /// Minted before dispatch so a canceller can hold the handle up front and
    /// cancel a call that has not reached the wire — such a handle must resolve
    /// as cancelled rather than start work.
    fn register_operation(&self) -> u64;

    /// Cancel the operation registered for `operation`, from any thread.
    /// Unknown or already-finished handles are ignored.
    fn cancel_operation(&self, operation: u64);

    /// Drop `operation`'s registration without cancelling it.
    ///
    /// [`Self::handle_message_cancellable`] already does this on every exit, so
    /// this is for the caller that registers a handle and then never dispatches
    /// it — an early error between minting and dispatch — which would otherwise
    /// leave an entry behind for the lifetime of the transport. Idempotent.
    fn deregister_operation(&self, operation: u64);

    /// Dispatch as [`Transport::handle_message`], but under `operation`, so a
    /// concurrent [`Self::cancel_operation`] reaches the running work.
    fn handle_message_cancellable(
        &self,
        service: &str,
        method: &str,
        message: Vec<u8>,
        operation: u64,
    ) -> impl Future<Output = Result<Vec<u8>, ProtoError<Vec<u8>>>> + Send;
}
