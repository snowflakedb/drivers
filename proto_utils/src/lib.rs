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
    ) -> impl std::future::Future<Output = Result<Vec<u8>, ProtoError<Vec<u8>>>> + Send;
}
