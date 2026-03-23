mod generator;
mod model;
mod parser;
mod replayer;

use std::path::PathBuf;
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
    /// Generate a C++ Catch2 test file from an ODBC trace log.
    Generate {
        /// Path to the ODBC trace log file.
        #[arg(short, long)]
        input: PathBuf,

        /// Path for the generated C++ test file.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Trace log format (auto-detected if omitted).
        #[arg(short, long, value_enum, default_value = "auto")]
        format: FormatArg,

        /// Test name used in the TEST_CASE_METHOD macro.
        #[arg(short = 'n', long, default_value = "trace replay")]
        test_name: String,

        /// Tag used in the Catch2 test (e.g. "[replay]").
        #[arg(short, long, default_value = "replay")]
        tag: String,
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
            format,
            test_name,
            tag,
        } => {
            let trace = parse_trace(&input, &format);
            let config = generator::cpp::GeneratorConfig { test_name, tag };
            let cpp_output = generator::cpp::generate(&trace, &config);

            match output {
                Some(path) => {
                    if let Err(e) = std::fs::write(&path, &cpp_output) {
                        eprintln!("Error writing output file: {e}");
                        process::exit(1);
                    }
                    println!("Generated test written to {}", path.display());
                }
                None => {
                    print!("{cpp_output}");
                }
            }
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

fn parse_trace(input: &std::path::Path, format: &FormatArg) -> model::TraceLog {
    let result = match format {
        FormatArg::Auto => parser::parse_file_auto(input),
        FormatArg::Iodbc => parser::parse_file(input, model::TraceFormat::IOdbc),
        FormatArg::Unixodbc => parser::parse_file(input, model::TraceFormat::UnixOdbc),
    };

    match result {
        Ok(trace) => trace,
        Err(e) => {
            eprintln!("Error parsing trace file: {e}");
            process::exit(1);
        }
    }
}
