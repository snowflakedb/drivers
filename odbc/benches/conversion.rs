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
//!
//! Cases cover every batched `CharKernel`: NUMBER, TIMESTAMP_NTZ,
//! TIMESTAMP_LTZ, TIMESTAMP_TZ (struct-encoded), DATE, TIME, and REAL, each
//! driving the generic `convert_char_range` loop through the type's kernel.
//! BOOLEAN layers `BooleanCharConverter`'s direct-write fast path on top of
//! its own `CharKernel`-backed converter; the narrow/wide-cell variants below
//! vary its stride.

use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BinaryArray, BooleanArray, Date32Array, Float64Array, Int32Array, Int64Array,
    StructArray,
};
use arrow::datatypes::{DataType, Field, Fields};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use odbc_sys as sql;
use sfodbc::bench_support::{
    Binding, BindingStrides, CDataType, ColumnConverter, ConversionError, Warnings, make_converter,
};

/// Rows per rowset — a typical block-cursor `SQL_ATTR_ROW_ARRAY_SIZE`.
const N: usize = 4096;
/// Bound char buffer width per row/cell.
const CELL: usize = 64;

fn field(arrow_type: DataType, logical_type: &str, extra: &[(&str, &str)]) -> Field {
    let mut md: HashMap<String, String> = HashMap::new();
    md.insert("logicalType".to_string(), logical_type.to_string());
    for (k, v) in extra {
        md.insert(k.to_string(), v.to_string());
    }
    Field::new("col", arrow_type, true).with_metadata(md)
}

/// TIMESTAMP_TZ arrives as a 2-column `Struct` on the wire (the scale 0-5
/// layout); build the matching field so `make_converter` routes it to the
/// batched struct kernel rather than the never-taken flat-`Int64` branch.
fn timestamp_tz_field() -> Field {
    let children = Fields::from(vec![
        Field::new("epoch", DataType::Int64, true),
        Field::new("tz_offset", DataType::Int32, true),
    ]);
    field(
        DataType::Struct(children),
        "TIMESTAMP_TZ",
        &[("scale", "0")],
    )
}

/// Build a full-rowset 2-column TIMESTAMP_TZ `StructArray` (no nulls). The
/// wire offset column is pre-biased by +1440 as Snowflake sends it; the
/// per-row offset varies to keep the render path honest.
fn tz_struct_array() -> StructArray {
    let epochs = Int64Array::from_iter_values((0..N as i64).map(|i| 1_700_000_000 + i * 37));
    let offsets = Int32Array::from_iter_values((0..N as i32).map(|i| 1440 + ((i % 27) - 13) * 30));
    let fields: Fields = vec![
        Arc::new(Field::new("epoch", DataType::Int64, true)),
        Arc::new(Field::new("tz_offset", DataType::Int32, true)),
    ]
    .into();
    StructArray::new(
        fields,
        vec![Arc::new(epochs) as ArrayRef, Arc::new(offsets) as ArrayRef],
        None,
    )
}

/// One `convert_arrow_range` pass over the whole rowset into a strided
/// `SQL_C_CHAR` buffer — exactly what `SQLFetch` drives per rowset.
fn run(
    conv: &dyn ColumnConverter,
    arr: &dyn Array,
    cell: usize,
    buf: &mut [u8],
    inds: &mut [sql::Len],
) {
    let base = Binding {
        target_type: CDataType::Char,
        target_value_ptr: buf.as_mut_ptr() as sql::Pointer,
        buffer_length: cell as sql::Len,
        octet_length_ptr: inds.as_mut_ptr(),
        indicator_ptr: inds.as_mut_ptr(),
        ..Default::default()
    };
    let mut outputs: Vec<Result<Warnings, ConversionError>> =
        (0..N).map(|_| Ok(Vec::new())).collect();
    conv.convert_arrow_range(
        black_box(arr),
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
    let cases: Vec<(&str, Field, ArrayRef)> = vec![
        (
            "timestamp_ntz",
            field(DataType::Int64, "TIMESTAMP_NTZ", &[("scale", "9")]),
            Arc::new(Int64Array::from_iter_values(
                (0..N as i64).map(|i| 1_700_000_000_000_000_000 + i * 1_000_000_000),
            )),
        ),
        (
            "timestamp_ltz",
            field(DataType::Int64, "TIMESTAMP_LTZ", &[("scale", "9")]),
            Arc::new(Int64Array::from_iter_values(
                (0..N as i64).map(|i| 1_700_000_000_000_000_000 + i * 1_000_000_000),
            )),
        ),
        (
            "timestamp_tz",
            timestamp_tz_field(),
            Arc::new(tz_struct_array()),
        ),
        (
            "number_12_2",
            field(
                DataType::Int64,
                "FIXED",
                &[("scale", "2"), ("precision", "12")],
            ),
            Arc::new(Int64Array::from_iter_values(
                (0..N as i64).map(|i| (i * 7919 % 90_000_000) + 101),
            )),
        ),
        (
            "boolean",
            field(DataType::Boolean, "BOOLEAN", &[]),
            Arc::new(BooleanArray::from(
                (0..N).map(|i| i % 2 == 0).collect::<Vec<bool>>(),
            )),
        ),
        (
            "date",
            field(DataType::Date32, "DATE", &[]),
            Arc::new(Date32Array::from_iter_values(
                (0..N as i32).map(|i| 19_000 + i % 3653),
            )),
        ),
        (
            "time",
            field(DataType::Int64, "TIME", &[("scale", "9")]),
            Arc::new(Int64Array::from_iter_values(
                (0..N as i64).map(|i| (i * 1_000_000_037) % 86_400_000_000_000),
            )),
        ),
        (
            "real",
            field(DataType::Float64, "REAL", &[]),
            Arc::new(Float64Array::from_iter_values(
                (0..N).map(|i| (i as f64) * 1.5 + 0.125),
            )),
        ),
    ];

    let mut group = c.benchmark_group("convert_arrow_range");
    group.throughput(Throughput::Elements(N as u64));
    for (name, f, arr) in &cases {
        let conv = make_converter(f);
        let mut buf = vec![0u8; N * 64];
        let mut inds = vec![0 as sql::Len; N];
        group.bench_function(*name, |b| {
            b.iter(|| run(conv.as_ref(), arr.as_ref(), CELL, &mut buf, &mut inds))
        });
    }
    let epoch_seconds: ArrayRef = Arc::new(Int64Array::from_iter_values(
        (0..N as i64).map(|i| 1_700_000_000 + i),
    ));
    let fractions: ArrayRef = Arc::new(Int32Array::from_iter_values(
        (0..N as i32).map(|i| (i * 97_531) % 1_000_000_000),
    ));
    let epoch_field = Arc::new(Field::new("epoch", DataType::Int64, false));
    let fraction_field = Arc::new(Field::new("fraction", DataType::Int32, false));
    let timestamp_struct = StructArray::from(vec![
        (Arc::clone(&epoch_field), Arc::clone(&epoch_seconds)),
        (Arc::clone(&fraction_field), Arc::clone(&fractions)),
    ]);
    let timestamp_struct_type = timestamp_struct.data_type().clone();

    let timestamp_ntz_struct_field = field(
        timestamp_struct_type.clone(),
        "TIMESTAMP_NTZ",
        &[("scale", "9")],
    );
    let timestamp_ntz_struct_converter = make_converter(&timestamp_ntz_struct_field);
    let mut timestamp_ntz_struct_buf = vec![0u8; N * 64];
    let mut timestamp_ntz_struct_inds = vec![0 as sql::Len; N];
    group.bench_function("timestamp_ntz_struct", |b| {
        b.iter(|| {
            run(
                timestamp_ntz_struct_converter.as_ref(),
                &timestamp_struct,
                64,
                &mut timestamp_ntz_struct_buf,
                &mut timestamp_ntz_struct_inds,
            )
        })
    });

    let offsets: ArrayRef = Arc::new(Int32Array::from_iter_values(
        (0..N as i32).map(|i| 1_440 + (i % 29 - 14) * 30),
    ));
    let offset_field = Arc::new(Field::new("tz_offset", DataType::Int32, false));
    let timestamp_tz_struct = StructArray::from(vec![
        (epoch_field, epoch_seconds),
        (fraction_field, fractions),
        (offset_field, offsets),
    ]);
    let timestamp_tz_struct_field = field(
        timestamp_tz_struct.data_type().clone(),
        "TIMESTAMP_TZ",
        &[("scale", "9")],
    );
    let timestamp_tz_struct_converter = make_converter(&timestamp_tz_struct_field);
    let mut timestamp_tz_struct_buf = vec![0u8; N * 64];
    let mut timestamp_tz_struct_inds = vec![0 as sql::Len; N];
    group.bench_function("timestamp_tz_struct", |b| {
        b.iter(|| {
            run(
                timestamp_tz_struct_converter.as_ref(),
                &timestamp_tz_struct,
                64,
                &mut timestamp_tz_struct_buf,
                &mut timestamp_tz_struct_inds,
            )
        })
    });

    let binary_field = field(DataType::Binary, "BINARY", &[("byteLength", "64")]);
    let binary_values: Vec<Vec<u8>> = (0..N)
        .map(|row| {
            (0..64)
                .map(|column| row.wrapping_mul(31).wrapping_add(column) as u8)
                .collect()
        })
        .collect();
    let binary_array = BinaryArray::from_iter_values(binary_values.iter().map(Vec::as_slice));
    let binary_converter = make_converter(&binary_field);
    let mut binary_buf = vec![0u8; N * 1024];
    let mut binary_inds = vec![0 as sql::Len; N];
    group.bench_function("binary_64_bytes", |b| {
        b.iter(|| {
            run(
                binary_converter.as_ref(),
                &binary_array,
                1024,
                &mut binary_buf,
                &mut binary_inds,
            )
        })
    });

    let boolean_field = field(DataType::Boolean, "BOOLEAN", &[]);
    let boolean_array = BooleanArray::from(
        (0..N)
            .map(|i| i.wrapping_mul(17) % 5 < 2)
            .collect::<Vec<_>>(),
    );
    let boolean_converter = make_converter(&boolean_field);
    for cell in [2, 64, 1024] {
        let mut buf = vec![0u8; N * cell];
        let mut inds = vec![0 as sql::Len; N];
        group.bench_function(format!("boolean_cell_{cell}"), |b| {
            b.iter(|| {
                run(
                    boolean_converter.as_ref(),
                    &boolean_array,
                    cell,
                    &mut buf,
                    &mut inds,
                )
            })
        });
    }

    let real_field = field(DataType::Float64, "REAL", &[]);
    let real_converter = make_converter(&real_field);

    // Mirrors TPCH L_EXTENDEDPRICE::DOUBLE: positive currency-like values
    // whose source column has two decimal places.
    let double_hundredths_array = Float64Array::from_iter_values(
        (0..N).map(|i| ((90_000 + i.wrapping_mul(7_919) % 100_000_000) as f64) / 100.0),
    );
    let mut double_hundredths_buf = vec![0u8; N * 64];
    let mut double_hundredths_inds = vec![0 as sql::Len; N];
    group.bench_function("double_hundredths", |b| {
        b.iter(|| {
            run(
                real_converter.as_ref(),
                &double_hundredths_array,
                64,
                &mut double_hundredths_buf,
                &mut double_hundredths_inds,
            )
        })
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
