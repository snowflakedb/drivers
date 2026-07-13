//! Criterion bench for the SQL_C_CHAR fetch-conversion hot path.
//!
//! Run with:
//! ```text
//! cargo bench -p odbc --features bench --bench conversion
//! ```
//!
//! Drives `ColumnConverter::convert_arrow_range` over a block-cursor rowset
//! bound as `SQL_C_CHAR` (the shape the perf harness and most ODBC apps use),
//! for the column types the perf PRs target. Replaces the earlier in-crate
//! `#[ignore]`d timing probe with a standard, statistically-rigorous bench.

use std::collections::HashMap;

use arrow::array::Int64Array;
use arrow::datatypes::{DataType, Field};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use odbc_sys as sql;
use sfodbc::bench_support::{
    Binding, BindingStrides, CDataType, ColumnConverter, ConversionError, Warnings, make_converter,
};

/// Rows per rowset — a typical block-cursor `SQL_ATTR_ROW_ARRAY_SIZE`.
const N: usize = 4096;
/// Bound char buffer width per row/cell.
const CELL: usize = 64;

fn field(logical_type: &str, extra: &[(&str, &str)]) -> Field {
    let mut md: HashMap<String, String> = HashMap::new();
    md.insert("logicalType".to_string(), logical_type.to_string());
    for (k, v) in extra {
        md.insert(k.to_string(), v.to_string());
    }
    Field::new("col", DataType::Int64, true).with_metadata(md)
}

/// One `convert_arrow_range` pass over the whole rowset into a strided
/// `SQL_C_CHAR` buffer — exactly what `SQLFetch` drives per rowset.
fn run(conv: &dyn ColumnConverter, arr: &Int64Array, buf: &mut [u8], inds: &mut [sql::Len]) {
    let base = Binding {
        target_type: CDataType::Char,
        target_value_ptr: buf.as_mut_ptr() as sql::Pointer,
        buffer_length: CELL as sql::Len,
        octet_length_ptr: inds.as_mut_ptr(),
        indicator_ptr: inds.as_mut_ptr(),
        ..Default::default()
    };
    let mut outputs: Vec<Result<Warnings, ConversionError>> =
        (0..N).map(|_| Ok(Vec::new())).collect();
    conv.convert_arrow_range(
        black_box(arr as &dyn arrow::array::Array),
        0..N,
        &base,
        0,
        BindingStrides {
            bind_type: 0,
            bind_offset: 0,
        },
        &mut outputs,
    );
    black_box(&buf);
}

fn bench(c: &mut Criterion) {
    let cases: [(&str, Field, Int64Array); 2] = [
        (
            "timestamp_ntz",
            field("TIMESTAMP_NTZ", &[("scale", "9")]),
            Int64Array::from_iter_values(
                (0..N as i64).map(|i| 1_700_000_000_000_000_000 + i * 1_000_000_000),
            ),
        ),
        (
            "number_12_2",
            field("FIXED", &[("scale", "2"), ("precision", "12")]),
            Int64Array::from_iter_values((0..N as i64).map(|i| (i * 7919 % 90_000_000) + 101)),
        ),
    ];

    let mut group = c.benchmark_group("convert_arrow_range");
    group.throughput(Throughput::Elements(N as u64));
    for (name, f, arr) in &cases {
        let conv = make_converter(f);
        let mut buf = vec![0u8; N * CELL];
        let mut inds = vec![0 as sql::Len; N];
        group.bench_function(*name, |b| {
            b.iter(|| run(conv.as_ref(), arr, &mut buf, &mut inds))
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
