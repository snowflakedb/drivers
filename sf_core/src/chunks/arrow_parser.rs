use std::io;

use arrow::array::RecordBatch;
use arrow::error::ArrowError;
use arrow_ipc::reader::StreamReader;

use super::prefetch::ParseChunk;

#[derive(Clone)]
pub struct ArrowChunkParser;

impl ParseChunk for ArrowChunkParser {
    fn parse_chunk(&self, data: Vec<u8>) -> Result<Vec<RecordBatch>, ArrowError> {
        let cursor = io::Cursor::new(data);
        let reader = StreamReader::try_new(cursor, None)?;
        reader.into_iter().collect::<Result<Vec<_>, _>>()
    }
}
