# Proto Generator

A Rust tool for generating protobuf interface files in the language of choice.

Available generators: `rust`, `python`, `java`, `json`.

## Usage

```
cargo run --bin proto_generator -- --generator <generator_name> -i <path_to_proto> -o <output_dir>
```

### Rust

Generated automatically during `cargo build` via `sf_core/build.rs` (invoked as a library, not the CLI). To run manually:

```
cargo run --bin proto_generator -- --generator rust \
  --input protobuf/database_driver_v1.proto \
  --output sf_core/src/protobuf_gen/
```

### Python

Generated automatically during `hatch build` / `pip install` via `python/hatch_build.py`. To run manually:

```
cargo run --bin proto_generator -- --generator python \
  --input protobuf/database_driver_v1.proto \
  --output python/src/snowflake/connector/_internal/protobuf_gen/
```

### Java

Generated automatically during `./gradlew build` via the `generateProtobuf` Gradle task. To run manually:

```
cargo run --bin proto_generator -- --generator java \
  --input protobuf/database_driver_v1.proto \
  --output jdbc/src/main/java/
```

### JSON

```
cargo run --bin proto_generator -- --generator json \
  --input protobuf/database_driver_v1.proto \
  --output <output_dir>
```
