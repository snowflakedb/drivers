use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use arrow::array::RecordBatchReader;
use arrow_ipc::reader::StreamReader;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

#[cfg(not(target_os = "windows"))]
use pprof::criterion::{Output, PProfProfiler};
#[cfg(not(target_os = "windows"))]
use pprof::flamegraph::Options as FlamegraphOptions;
use sf_core::chunks::ChunkDownloadData;
use sf_core::chunks::mock::FileChunkDownloader;
use sf_core::chunks::prefetch::{ArrowChunkParser, JsonChunkParser, PrefetchChunkReader};
use sf_core::query_types::RowType;

const DEFAULT_TEXT_MAX_LENGTH: u64 = 16_777_216; // 16 MiB
const DEFAULT_BINARY_MAX_LENGTH: u64 = 8_388_608; // 8 MiB

#[derive(serde::Deserialize)]
struct Metadata {
    format: String,
    chunk_count: usize,
    has_initial: bool,
    #[serde(default)]
    total_rows: Option<i64>,
    row_types: Option<Vec<RowTypeMeta>>,
    #[serde(default)]
    chunks: Vec<ChunkMeta>,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct ChunkMeta {
    index: usize,
    row_count: i32,
    uncompressed_size: i64,
    compressed_size: i64,
    #[serde(default)]
    saved_size: u64,
}

#[derive(serde::Deserialize)]
struct RowTypeMeta {
    name: String,
    #[serde(rename = "type")]
    type_: String,
    nullable: bool,
    scale: Option<u64>,
    precision: Option<u64>,
    length: Option<u64>,
    byte_length: Option<u64>,
}

impl RowTypeMeta {
    fn to_row_type(&self) -> RowType {
        match self.type_.to_uppercase().as_str() {
            "FIXED" => RowType::fixed(
                &self.name,
                self.nullable,
                self.precision.unwrap_or(38),
                self.scale.unwrap_or(0),
            ),
            "TEXT" => RowType::text(
                &self.name,
                self.nullable,
                self.length.unwrap_or(DEFAULT_TEXT_MAX_LENGTH),
                self.byte_length.unwrap_or(DEFAULT_TEXT_MAX_LENGTH),
            ),
            "REAL" => RowType::real(&self.name, self.nullable),
            "BOOLEAN" => RowType::boolean(&self.name, self.nullable),
            "DATE" => RowType::date(&self.name, self.nullable),
            "TIMESTAMP_NTZ" => {
                RowType::timestamp_ntz(&self.name, self.nullable, self.scale.unwrap_or(9))
            }
            "TIMESTAMP_LTZ" => {
                RowType::timestamp_ltz(&self.name, self.nullable, self.scale.unwrap_or(9))
            }
            "TIMESTAMP_TZ" => {
                RowType::timestamp_tz(&self.name, self.nullable, self.scale.unwrap_or(9))
            }
            "TIME" => RowType::time(&self.name, self.nullable, self.scale.unwrap_or(9)),
            "BINARY" => RowType::binary(
                &self.name,
                self.nullable,
                self.length.unwrap_or(DEFAULT_BINARY_MAX_LENGTH),
                self.byte_length.unwrap_or(DEFAULT_BINARY_MAX_LENGTH),
            ),
            "VARIANT" => RowType::variant(&self.name, self.nullable),
            "OBJECT" => RowType::object(&self.name, self.nullable),
            "ARRAY" => RowType::array(&self.name, self.nullable),
            other => panic!("unsupported row type in metadata: {other}"),
        }
    }
}

fn repo_root() -> PathBuf {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("failed to run git");
    let root = String::from_utf8(output.stdout).expect("invalid utf8 from git");
    PathBuf::from(root.trim())
}

fn default_chunks_dir() -> PathBuf {
    repo_root()
        .join("tests")
        .join("test_data")
        .join("generated_test_data")
        .join("chunks")
}

fn chunks_base_dir() -> PathBuf {
    std::env::var("CHUNK_TEST_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_chunks_dir())
}

fn load_metadata(dir: &Path) -> Metadata {
    let path = dir.join("metadata.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()))
}

fn build_chunk_download_data(dir: &Path, metadata: &Metadata) -> VecDeque<ChunkDownloadData> {
    (0..metadata.chunk_count)
        .map(|i| {
            let path = dir.join(format!("chunk_{i}.bin"));
            let chunk_meta = metadata.chunks.iter().find(|c| c.index == i);
            ChunkDownloadData {
                url: path.to_string_lossy().into_owned(),
                row_count: chunk_meta.map_or(0, |c| c.row_count),
                uncompressed_size: chunk_meta.map_or(0, |c| c.uncompressed_size),
                compressed_size: chunk_meta.map_or(0, |c| c.compressed_size),
                headers: HashMap::new(),
            }
        })
        .collect()
}

fn drain_reader(reader: &mut dyn RecordBatchReader) -> usize {
    let mut total_rows = 0;
    for batch_result in reader {
        let batch = batch_result.expect("failed to read batch");
        total_rows += batch.num_rows();
    }
    total_rows
}

fn bench_arrow_prefetch(c: &mut Criterion) {
    let dir = chunks_base_dir().join("arrow_1M_15columns");
    let metadata_path = dir.join("metadata.json");
    if !metadata_path.exists() {
        eprintln!(
            "Skipping arrow_prefetch benchmark: no test data at {}",
            dir.display()
        );
        eprintln!("Run generate_chunks.sh first to generate test data.");
        return;
    }

    let metadata = load_metadata(&dir);
    if metadata.format != "arrow" {
        eprintln!(
            "Skipping arrow_prefetch benchmark: test data format is '{}', expected 'arrow'",
            metadata.format
        );
        return;
    }

    let initial_path = dir.join("initial.bin");
    let has_initial = metadata.has_initial && initial_path.exists();

    let total_chunk_rows: i64 = metadata.chunks.iter().map(|c| c.row_count as i64).sum();
    if let Some(total) = metadata.total_rows {
        println!(
            "Arrow benchmark: {total} total rows, {} chunks, {total_chunk_rows} rows in chunks",
            metadata.chunk_count
        );
    }

    let mut group = c.benchmark_group("arrow_prefetch");

    for concurrency in [1, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("concurrency", concurrency),
            &concurrency,
            |b, &concurrency| {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build tokio runtime");

                b.iter(|| {
                    let mut chunks = build_chunk_download_data(&dir, &metadata);

                    let initial_reader = if has_initial {
                        let bytes = std::fs::read(&initial_path).expect("failed to read initial");
                        let cursor = io::Cursor::new(bytes);
                        StreamReader::try_new(cursor, None).expect("failed to parse initial IPC")
                    } else {
                        let first = chunks.pop_front().expect("no chunks available");
                        let bytes = std::fs::read(&first.url).expect("failed to read first chunk");
                        let cursor = io::Cursor::new(bytes);
                        StreamReader::try_new(cursor, None).expect("failed to parse first chunk")
                    };

                    let downloader = FileChunkDownloader;
                    let parser = ArrowChunkParser;
                    let mut reader = rt
                        .block_on(PrefetchChunkReader::reader(
                            initial_reader,
                            chunks,
                            downloader,
                            parser,
                            concurrency,
                        ))
                        .expect("failed to create prefetch reader");

                    drain_reader(reader.as_mut())
                });
            },
        );
    }

    group.finish();
}

fn bench_json_prefetch(c: &mut Criterion) {
    let dir = chunks_base_dir().join("json_1M_15columns");
    let metadata_path = dir.join("metadata.json");
    if !metadata_path.exists() {
        eprintln!(
            "Skipping json_prefetch benchmark: no test data at {}",
            dir.display()
        );
        return;
    }

    let metadata = load_metadata(&dir);
    if metadata.format != "json" {
        eprintln!(
            "Skipping json_prefetch benchmark: test data format is '{}', expected 'json'",
            metadata.format
        );
        return;
    }

    let row_type_metas = metadata
        .row_types
        .as_ref()
        .expect("JSON metadata must have row_types");
    let row_types: Vec<RowType> = row_type_metas.iter().map(|rt| rt.to_row_type()).collect();

    let initial_path = dir.join("initial.bin");
    let has_initial = metadata.has_initial && initial_path.exists();

    let initial_rowset: Vec<Vec<Option<String>>> = if has_initial {
        let bytes = std::fs::read(&initial_path).expect("failed to read initial");
        serde_json::from_slice(&bytes).expect("failed to parse initial JSON rowset")
    } else {
        vec![]
    };

    let initial_reader =
        sf_core::arrow_utils::convert_string_rowset_to_arrow_reader(&initial_rowset, &row_types)
            .expect("failed to convert initial rowset");

    let initial_schema = initial_reader.schema();
    let initial_batches: Vec<_> = initial_reader
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("failed to read initial batches");

    let total_chunk_rows: i64 = metadata.chunks.iter().map(|c| c.row_count as i64).sum();
    if let Some(total) = metadata.total_rows {
        println!(
            "JSON benchmark: {total} total rows, {} chunks, {total_chunk_rows} rows in chunks",
            metadata.chunk_count
        );
    }

    let mut group = c.benchmark_group("json_prefetch");

    for concurrency in [1, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("concurrency", concurrency),
            &concurrency,
            |b, &concurrency| {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build tokio runtime");

                b.iter(|| {
                    let chunks = build_chunk_download_data(&dir, &metadata);

                    let initial_reader = arrow::array::RecordBatchIterator::new(
                        initial_batches.clone().into_iter().map(Ok),
                        initial_schema.clone(),
                    );

                    let downloader = FileChunkDownloader;
                    let parser = JsonChunkParser {
                        row_types: row_types.clone(),
                    };
                    let mut reader = rt
                        .block_on(PrefetchChunkReader::reader(
                            initial_reader,
                            chunks,
                            downloader,
                            parser,
                            concurrency,
                        ))
                        .expect("failed to create prefetch reader");

                    drain_reader(reader.as_mut())
                });
            },
        );
    }

    group.finish();
}

#[cfg(not(target_os = "windows"))]
fn profiled_config() -> Criterion {
    let mut opts = FlamegraphOptions::default();
    opts.reverse_stack_order = true;
    let profiler = PProfProfiler::new(100, Output::Flamegraph(Some(opts)));
    Criterion::default().with_profiler(profiler).sample_size(10)
}

#[cfg(target_os = "windows")]
fn profiled_config() -> Criterion {
    Criterion::default().sample_size(10)
}

criterion_group!(
    name = benches;
    config = profiled_config();
    targets = bench_arrow_prefetch, bench_json_prefetch
);
criterion_main!(benches);
