#[derive(Debug)]
pub enum ProtoError<T> {
    Transport(String),
    Application(T),
}

pub trait Transport {
    async fn handle_message(
        &self,
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>>;
}
