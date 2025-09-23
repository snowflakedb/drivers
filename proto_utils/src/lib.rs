#[derive(Debug)]
pub enum ProtoError<T> {
    Transport(String),
    Application(T),
}

pub trait Transport {
    fn handle_message(
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>>;
}
