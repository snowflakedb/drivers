use arrow::array::RecordBatch;
use arrow::buffer::Buffer;
use arrow::error::ArrowError;
use arrow_ipc::reader::StreamDecoder;

use super::prefetch::ParseChunk;

#[derive(Clone)]
pub struct ArrowChunkParser;

impl ParseChunk for ArrowChunkParser {
    /// Decodes the IPC stream in place: record batch buffers slice the chunk
    /// allocation instead of being copied out of it, so a chunk occupies its
    /// decoded size once rather than twice while it is parsed.
    #[tracing::instrument(
        name = "core_arrow_decode",
        target = "sf_core::perf",
        level = "trace",
        skip_all
    )]
    fn parse_chunk(&self, data: Vec<u8>) -> Result<Vec<RecordBatch>, ArrowError> {
        let mut buffer = Buffer::from_vec(data);
        let mut decoder = StreamDecoder::new();
        let mut batches = Vec::new();
        while !buffer.is_empty() {
            if let Some(batch) = decoder.decode(&mut buffer)? {
                batches.push(batch);
            }
        }
        decoder.finish()?;
        Ok(batches)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{Array, Int64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::ipc::writer::StreamWriter;

    use super::*;

    fn ipc_stream(batches: &[RecordBatch]) -> Vec<u8> {
        let mut writer = StreamWriter::try_new(Vec::new(), &batches[0].schema()).unwrap();
        for batch in batches {
            writer.write(batch).unwrap();
        }
        writer.into_inner().unwrap()
    }

    fn batch(ids: Vec<Option<i64>>, names: Vec<Option<&str>>) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("ID", DataType::Int64, true),
            Field::new("NAME", DataType::Utf8, true),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int64Array::from(ids)),
                Arc::new(StringArray::from(names)),
            ],
        )
        .unwrap()
    }

    #[test]
    fn parses_every_batch_without_copying_values_out_of_the_chunk() {
        let expected = vec![
            batch(
                vec![Some(1), None, Some(3)],
                vec![Some("a"), Some("bb"), None],
            ),
            batch(vec![Some(4)], vec![Some("dddd")]),
        ];
        let data = ipc_stream(&expected);
        let chunk = data.as_ptr_range();
        let chunk = chunk.start as usize..chunk.end as usize;

        let batches = ArrowChunkParser.parse_chunk(data).unwrap();

        assert_eq!(batches, expected);
        for batch in &batches {
            let values = batch.column(0).to_data().buffers()[0].as_ptr() as usize;
            assert!(
                chunk.contains(&values),
                "values were copied out of the chunk"
            );
        }
    }

    #[test]
    fn schema_only_stream_yields_no_batches() {
        let schema = batch(vec![], vec![]).schema();
        let data = StreamWriter::try_new(Vec::new(), &schema)
            .unwrap()
            .into_inner()
            .unwrap();
        assert!(ArrowChunkParser.parse_chunk(data).unwrap().is_empty());
    }

    #[test]
    fn truncated_stream_is_an_error() {
        let mut data = ipc_stream(&[batch(vec![Some(1)], vec![Some("a")])]);
        data.truncate(data.len() - 16);
        assert!(ArrowChunkParser.parse_chunk(data).is_err());
    }
}
