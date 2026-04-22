mod compare;
mod generator;
mod ir;
mod model;
mod parser;
mod query_map;
mod replayer;
#[allow(dead_code)]
mod splitter;

use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "odbc-trace-tool")]
#[command(about = "Parse ODBC trace logs to generate C++ Catch2 tests or replay against a driver")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a C++ Catch2 test file from a trace log or IR YAML.
    Generate {
        /// Path to an ODBC trace log or IR YAML file (.yaml/.yml).
        #[arg(short, long)]
        input: PathBuf,

        /// Path for the generated C++ test file (default: <input_dir>/test.cpp).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Path to a YAML query mapping file (default: <input_dir>/queries.yaml).
        /// Auto-created with DTM-stripped defaults if it does not exist.
        #[arg(short = 'q', long)]
        query_map: Option<PathBuf>,

        /// Trace log format (auto-detected if omitted; ignored for IR YAML input).
        #[arg(short, long, value_enum, default_value = "auto")]
        format: FormatArg,

        /// Test name used in the TEST_CASE_METHOD macro.
        #[arg(short = 'n', long, default_value = "trace replay")]
        test_name: String,

        /// Tag used in the Catch2 test (e.g. "[replay]").
        #[arg(short, long, default_value = "replay")]
        tag: String,
    },

    /// Extract SQL queries from a trace and write a YAML mapping file.
    QueryMap {
        /// Path to the ODBC trace log file.
        #[arg(short, long)]
        input: PathBuf,

        /// Path for the generated query mapping file (default: <input_dir>/queries.yaml).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Trace log format (auto-detected if omitted).
        #[arg(short, long, value_enum, default_value = "auto")]
        format: FormatArg,
    },

    /// Split an ODBC trace into per-handle IR YAML files.
    Split {
        /// Path to an ODBC trace log or IR YAML file.
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for split IR YAML files.
        #[arg(short, long)]
        output_dir: PathBuf,

        /// Split granularity.
        #[arg(short, long, value_enum)]
        mode: ir::SplitMode,

        /// Skip traces that contain truncated SQL strings.
        #[arg(long, default_value = "false")]
        require_complete_sql: bool,

        /// Trace log format (auto-detected if omitted; ignored for IR YAML input).
        #[arg(short, long, value_enum, default_value = "auto")]
        format: FormatArg,
    },

    /// Dump the handle-tree intermediate representation for debugging.
    DumpIr {
        /// Path to the ODBC trace log file.
        #[arg(short, long)]
        input: PathBuf,

        /// Write YAML to a file instead of human-readable text to stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Trace log format (auto-detected if omitted).
        #[arg(short, long, value_enum, default_value = "auto")]
        format: FormatArg,
    },

    /// Compare traces by edit distance on their flattened call sequences.
    Compare {
        /// One or more reference traces. Each input is compared against all
        /// references and the minimum distance is reported.
        #[arg(short, long, num_args = 1..)]
        reference: Vec<PathBuf>,

        /// Traces to compare against the references.
        #[arg(short, long, num_args = 1..)]
        inputs: Vec<PathBuf>,

        /// Trace log format (auto-detected if omitted; ignored for IR YAML input).
        #[arg(short, long, value_enum, default_value = "auto")]
        format: FormatArg,
    },

    /// Replay an ODBC trace log against the driver, comparing results.
    Replay {
        /// Path to the ODBC trace log file.
        #[arg(short, long)]
        input: PathBuf,

        /// ODBC connection string to use (overrides the trace's DSN).
        #[arg(short, long)]
        connection_string: String,

        /// Trace log format (auto-detected if omitted).
        #[arg(short, long, value_enum, default_value = "auto")]
        format: FormatArg,

        /// Accept SQL_SUCCESS_WITH_INFO where SQL_SUCCESS was expected (and vice versa).
        #[arg(long, default_value = "true")]
        relaxed: bool,
    },
}

#[derive(Clone, ValueEnum)]
enum FormatArg {
    Auto,
    Iodbc,
    Unixodbc,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            input,
            output,
            query_map: query_map_path,
            format,
            test_name,
            tag,
        } => {
            let calls = load_calls(&input, &format);

            let qm_path =
                query_map_path.unwrap_or_else(|| default_sibling_path(&input, "queries.yaml"));

            let qm = load_or_create_query_map(&qm_path, &calls);

            let config = generator::cpp::GeneratorConfig {
                test_name,
                tag,
                query_map: Some(qm),
            };
            let cpp_output = generator::cpp::generate(&calls, &config);

            let out_path = output.unwrap_or_else(|| default_sibling_path(&input, "test.cpp"));

            if let Err(e) = std::fs::write(&out_path, &cpp_output) {
                eprintln!("Error writing output file: {e}");
                process::exit(1);
            }
            println!("Generated test written to {}", out_path.display());
        }
        Commands::Split {
            input,
            output_dir,
            mode,
            require_complete_sql,
            format,
        } => {
            let trace_ir = load_ir(&input, &format);
            let all_parts = trace_ir.split(mode);

            let parts: Vec<_> = if require_complete_sql {
                all_parts
                    .into_iter()
                    .filter(|(_, sub)| !sub.roots.iter().any(|r| r.has_truncated_sql()))
                    .collect()
            } else {
                all_parts
            };

            if let Err(e) = std::fs::create_dir_all(&output_dir) {
                eprintln!("Error creating output directory: {e}");
                process::exit(1);
            }

            for (name, sub_ir) in &parts {
                let dir = output_dir.join(name);
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    eprintln!("Error creating directory {}: {e}", dir.display());
                    process::exit(1);
                }
                let path = dir.join("ir.yaml");
                let yaml = serde_yaml::to_string(sub_ir).unwrap_or_else(|e| {
                    eprintln!("Error serializing IR for {name}: {e}");
                    process::exit(1);
                });
                if let Err(e) = std::fs::write(&path, &yaml) {
                    eprintln!("Error writing {}: {e}", path.display());
                    process::exit(1);
                }
            }

            println!(
                "Split complete: {} IR files written to {}",
                parts.len(),
                output_dir.display()
            );
        }
        Commands::QueryMap {
            input,
            output,
            format,
        } => {
            let calls = load_calls(&input, &format);
            let qm = query_map::generate_query_map(&calls);
            let out_path = output.unwrap_or_else(|| default_sibling_path(&input, "queries.yaml"));

            let yaml = serde_yaml::to_string(&qm).unwrap_or_else(|e| {
                eprintln!("Error serializing query map: {e}");
                process::exit(1);
            });

            if let Err(e) = std::fs::write(&out_path, &yaml) {
                eprintln!("Error writing query map: {e}");
                process::exit(1);
            }
            println!(
                "Query map written to {} ({} queries)",
                out_path.display(),
                qm.queries.len()
            );
        }
        Commands::DumpIr {
            input,
            output,
            format,
        } => {
            let trace_ir = load_ir(&input, &format);

            if let Some(out_path) = output {
                let yaml = serde_yaml::to_string(&trace_ir).unwrap_or_else(|e| {
                    eprintln!("Error serializing IR: {e}");
                    process::exit(1);
                });
                if let Err(e) = std::fs::write(&out_path, &yaml) {
                    eprintln!("Error writing IR: {e}");
                    process::exit(1);
                }
                println!(
                    "IR written to {} ({} operations, {} handles)",
                    out_path.display(),
                    trace_ir.total_operations,
                    trace_ir.handle_count()
                );
            } else {
                print!("{trace_ir}");
            }
        }
        Commands::Compare {
            reference,
            inputs,
            format,
        } => {
            let ref_data: Vec<_> = reference
                .iter()
                .map(|p| {
                    let ir = load_ir(p, &format);
                    let calls = ir.flatten_calls();
                    let name = p.display().to_string();
                    (name, calls)
                })
                .collect();

            let mut results = Vec::new();
            for path in &inputs {
                let ir = load_ir(path, &format);
                let calls = ir.flatten_calls();
                let filtered = compare::filter_for_comparison(&calls);

                let best = ref_data
                    .iter()
                    .map(|(name, ref_calls)| {
                        let ref_filtered = compare::filter_for_comparison(ref_calls);
                        let d = compare::compare_traces(&ref_filtered, &filtered);
                        (name.as_str(), d, ref_filtered.len())
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .unwrap();

                results.push(compare::CompareResult {
                    name_a: best.0.to_string(),
                    name_b: path.display().to_string(),
                    distance: best.1,
                    len_b: filtered.len(),
                });
            }

            let ref_label = if ref_data.len() == 1 {
                ref_data[0].0.clone()
            } else {
                format!("{} references", ref_data.len())
            };
            compare::print_report(&ref_label, &mut results);
        }
        Commands::Replay {
            input,
            connection_string,
            format,
            relaxed,
        } => {
            let trace = parse_trace(&input, &format);
            let config = replayer::ReplayConfig {
                connection_string,
                relaxed_success: relaxed,
            };

            match replayer::replay(&trace, &config) {
                Ok(summary) => {
                    replayer::print_report(&summary);
                    if !summary.all_passed() {
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Replay error: {e}");
                    process::exit(1);
                }
            }
        }
    }
}

fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .map(|e| e == "yaml" || e == "yml")
        .unwrap_or(false)
}

/// Load an IR, either from a YAML file or by parsing a trace log and building it.
fn load_ir(input: &Path, format: &FormatArg) -> ir::TraceIr {
    if is_yaml_file(input) {
        ir::load_ir_yaml(input).unwrap_or_else(|e| {
            eprintln!("Error loading IR YAML {}: {e}", input.display());
            process::exit(1);
        })
    } else {
        let trace = parse_trace(input, format);
        ir::build_ir(&trace)
    }
}

/// Load a flat call list, either from IR YAML or by parsing a trace log.
fn load_calls(input: &Path, format: &FormatArg) -> Vec<model::OdbcCall> {
    if is_yaml_file(input) {
        let trace_ir = ir::load_ir_yaml(input).unwrap_or_else(|e| {
            eprintln!("Error loading IR YAML {}: {e}", input.display());
            process::exit(1);
        });
        trace_ir.flatten_calls()
    } else {
        let trace = parse_trace(input, format);
        trace.calls.into_iter().map(|tc| tc.call).collect()
    }
}

fn default_sibling_path(input: &Path, filename: &str) -> PathBuf {
    input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(filename)
}

fn load_or_create_query_map(path: &Path, calls: &[model::OdbcCall]) -> query_map::QueryMap {
    if path.exists() {
        let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Error reading query map {}: {e}", path.display());
            process::exit(1);
        });
        let qm: query_map::QueryMap = serde_yaml::from_str(&content).unwrap_or_else(|e| {
            eprintln!("Error parsing query map {}: {e}", path.display());
            process::exit(1);
        });
        println!(
            "Loaded query map from {} ({} queries)",
            path.display(),
            qm.queries.len()
        );
        qm
    } else {
        let qm = query_map::generate_query_map(calls);
        let yaml = serde_yaml::to_string(&qm).unwrap_or_else(|e| {
            eprintln!("Error serializing query map: {e}");
            process::exit(1);
        });
        if let Err(e) = std::fs::write(path, &yaml) {
            eprintln!("Error writing query map {}: {e}", path.display());
            process::exit(1);
        }
        println!(
            "Created query map at {} ({} queries)",
            path.display(),
            qm.queries.len()
        );
        qm
    }
}

fn parse_trace(input: &std::path::Path, format: &FormatArg) -> model::TraceLog {
    let result = match format {
        FormatArg::Auto => parser::parse_file_auto(input),
        FormatArg::Iodbc => parser::parse_file(input, model::TraceFormat::IOdbc),
        FormatArg::Unixodbc => parser::parse_file(input, model::TraceFormat::UnixOdbc),
    };

    match result {
        Ok(mut trace) => {
            trace.header.source_file = Some(input.display().to_string());
            trace
        }
        Err(e) => {
            eprintln!("Error parsing trace file: {e}");
            process::exit(1);
        }
    }
}
