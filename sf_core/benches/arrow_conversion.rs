use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use sf_core::arrow_utils::convert_string_rowset_to_arrow_reader;
use sf_core::query_types::RowType;

fn bench_small_rowset(c: &mut Criterion) {
    let rowset = vec![
        vec!["alpha.txt".to_string(), "7".to_string()],
        vec!["beta.md".to_string(), "123".to_string()],
        vec!["gamma.bin".to_string(), "32767".to_string()],
        vec!["delta.png".to_string(), "1024".to_string()],
    ];

    let row_types = vec![
        RowType::text("col_text", false, 16, 64),
        RowType::fixed("col_fixed", false, 5, 0).unwrap(),
    ];

    c.bench_function("convert_small_rowset", |b| {
        b.iter(|| {
            let _reader =
                convert_string_rowset_to_arrow_reader(black_box(&rowset), black_box(&row_types))
                    .unwrap();
        })
    });
}

fn bench_medium_rowset(c: &mut Criterion) {
    // 1000 rows
    let mut rowset = Vec::new();
    for i in 0..1000 {
        rowset.push(vec![format!("file_{}.txt", i), i.to_string()]);
    }

    let row_types = vec![
        RowType::text("filename", false, 64, 256),
        RowType::fixed("size", false, 19, 0).unwrap(),
    ];

    c.bench_function("convert_medium_rowset_1k", |b| {
        b.iter(|| {
            let _reader =
                convert_string_rowset_to_arrow_reader(black_box(&rowset), black_box(&row_types))
                    .unwrap();
        })
    });
}

fn bench_large_rowset(c: &mut Criterion) {
    let mut group = c.benchmark_group("convert_large_rowset");

    for size in [10_000, 50_000, 100_000].iter() {
        let mut rowset = Vec::new();
        for i in 0..*size {
            rowset.push(vec![format!("file_{}.txt", i), i.to_string()]);
        }

        let row_types = vec![
            RowType::text("filename", false, 64, 256),
            RowType::fixed("size", false, 19, 0).unwrap(),
        ];

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let _reader = convert_string_rowset_to_arrow_reader(
                    black_box(&rowset),
                    black_box(&row_types),
                )
                .unwrap();
            })
        });
    }
    group.finish();
}

fn bench_decimal_conversion(c: &mut Criterion) {
    let mut rowset = Vec::new();
    for i in 0..1000 {
        rowset.push(vec![format!("{}.{:02}", i / 100, i % 100)]);
    }

    let row_types = vec![RowType::fixed("amount", false, 10, 2).unwrap()];

    c.bench_function("convert_decimal_1k", |b| {
        b.iter(|| {
            let _reader =
                convert_string_rowset_to_arrow_reader(black_box(&rowset), black_box(&row_types))
                    .unwrap();
        })
    });
}

fn bench_large_integer_fallback(c: &mut Criterion) {
    let mut rowset = Vec::new();
    for i in 0..1000 {
        if i % 100 == 0 {
            // Every 100th value is too large for i64
            rowset.push(vec!["99999999999999999999999999999999999999".to_string()]);
        } else {
            rowset.push(vec![i.to_string()]);
        }
    }

    let row_types = vec![RowType::fixed("big_num", false, 38, 0).unwrap()];

    c.bench_function("convert_with_large_int_fallback", |b| {
        b.iter(|| {
            let _reader =
                convert_string_rowset_to_arrow_reader(black_box(&rowset), black_box(&row_types))
                    .unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_small_rowset,
    bench_medium_rowset,
    bench_large_rowset,
    bench_decimal_conversion,
    bench_large_integer_fallback
);
criterion_main!(benches);
