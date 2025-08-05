use std::io::{Read, Write};
use std::sync::Mutex;
use thrift::protocol::{TCompactInputProtocol, TCompactOutputProtocol};
use thrift::server::TProcessor;
use tracing::{Level, event, span, trace};

use crate::handle_manager::HandleManager;

pub static TRANSPORT_HANDLE_MANAGER: HandleManager<Mutex<ThriftTransport>> = HandleManager::new();

pub struct ThriftTransport {
    id: u64,
    input: Buffer,
    processor: Box<dyn TProcessor + Send + Sync>,
    output: Buffer,
}

struct Buffer {
    bytes: Vec<u8>,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer { bytes: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
    }
}

impl std::io::Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = buf.len().min(self.bytes.len());
        buf[..len].copy_from_slice(&self.bytes[..len]);
        self.bytes.drain(..len);
        Ok(len)
    }
}

impl std::io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ThriftTransport {
    pub fn new(processor: Box<dyn TProcessor + Send + Sync>) -> Self {
        ThriftTransport {
            id: rand::random::<u64>(),
            input: Buffer::new(),
            processor,
            output: Buffer::new(),
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> thrift::Result<usize> {
        let span =
            span!(target: "thrift_transport", Level::TRACE, "ThriftTransport::write", id=?self.id);
        let _enter = span.enter();
        match self.input.write(buf) {
            Ok(len) => {
                event!(target: "thrift_transport", Level::TRACE, "Wrote {} bytes to transport", len);
                Ok(len)
            }
            Err(e) => {
                event!(target: "thrift_transport", Level::ERROR, "Error writing to transport: {:?}", e);
                Err(thrift::Error::from(e))
            }
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> thrift::Result<usize> {
        let span =
            span!(target: "thrift_transport", Level::TRACE, "ThriftTransport::read", id=?self.id);
        let _enter = span.enter();
        match self.output.read(buf) {
            Ok(len) => {
                event!(target: "thrift_transport", Level::TRACE, "Read {} bytes from transport", len);
                Ok(len)
            }
            Err(e) => {
                event!(target: "thrift_transport", Level::ERROR, "Error reading from transport: {:?}", e);
                Err(thrift::Error::from(e))
            }
        }
    }
    pub fn flush(&mut self) -> thrift::Result<()> {
        let span =
            span!(target: "thrift_transport", Level::TRACE, "ThriftTransport::flush", id=?self.id);
        let _enter = span.enter();
        trace!(target: "thrift_transport", "Clearing output buffer");
        self.output.clear();
        let input_bytes = self.input.bytes.len();
        let mut input_protocol = TCompactInputProtocol::new(&mut self.input);
        let mut output_protocol = TCompactOutputProtocol::new(&mut self.output);
        trace!(target: "thrift_transport", "Processing a call(input_length={:?})", input_bytes);
        self.processor
            .process(&mut input_protocol, &mut output_protocol)?;
        trace!(target: "thrift_transport", "Finished processing a call(output_length={:?})", self.output.bytes.len());
        trace!(target: "thrift_transport", "Clearing input buffer");
        self.input.clear();
        Ok(())
    }
}
