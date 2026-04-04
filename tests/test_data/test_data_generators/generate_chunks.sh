#!/usr/bin/env bash
set -euo pipefail
PROJECT_ROOT=$(git rev-parse --show-toplevel)
PARAMETER_PATH=${PARAMETER_PATH:-$PROJECT_ROOT/parameters.json}

CHUNKS_DIR=$PROJECT_ROOT/tests/test_data/generated_test_data/chunks

mkdir -p "$CHUNKS_DIR"

echo "Collecting chunk data..."

pushd "$PROJECT_ROOT"
    QUERY_1M_15columns="SELECT
                    L_ORDERKEY,
                    L_PARTKEY,
                    L_SUPPKEY,
                    L_LINENUMBER,
                    L_QUANTITY,
                    L_EXTENDEDPRICE,
                    L_DISCOUNT,
                    L_TAX,
                    L_RETURNFLAG,
                    L_LINESTATUS,
                    L_SHIPDATE,
                    L_COMMITDATE,
                    L_RECEIPTDATE,
                    L_SHIPINSTRUCT,
                    L_COMMENT
                FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM
                LIMIT 1000000"

    cargo run --bin collect_chunk_data -- \
        --parameter-path "$PARAMETER_PATH" \
        --output-dir "$CHUNKS_DIR/arrow_1M_15columns" \
        --format arrow \
        --sql "$QUERY_1M_15columns"

    cargo run --bin collect_chunk_data -- \
        --parameter-path "$PARAMETER_PATH" \
        --output-dir "$CHUNKS_DIR/json_1M_15columns" \
        --format json \
        --sql "$QUERY_1M_15columns"
popd

echo "Done"
