use clap::{Arg, ArgMatches, Command};
use proto_generator::{CodeGenerator, GenerationResult, GeneratorContext};
use proto_generator::{JsonGenerator, PythonGenerator, RustGenerator};
use snafu::Whatever;
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;

lazy_static! {
    static ref generators: Vec<Box<dyn CodeGenerator + Sync>> = vec![
        Box::new(RustGenerator::new()),
        Box::new(JsonGenerator::new()),
        Box::new(PythonGenerator::new())
    ];
}

fn main() -> Result<(), Whatever> {
    env_logger::init();
    let matches = build_cli().get_matches();

    handle_generate_command(&matches)?;
    Ok(())
}

fn build_cli() -> Command {
    Command::new("proto_generator")
        .version("0.1.0")
        .author("Snowflake Inc.")
        .about("A tool for parsing protobuf files and generating custom service implementations")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILE")
                .help("Input protobuf file")
                .required(true),
        )
        .arg(
            Arg::new("generator")
                .short('g')
                .long("generator")
                .value_name("GENERATOR")
                .help("Generator to use (list available with 'info generators')")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("DIR")
                .help("Output directory")
                .default_value("./generated"),
        )
        .arg(
            Arg::new("option")
                .long("option")
                .value_name("KEY=VALUE")
                .help("Generator-specific option")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Verbose output"),
        )
}

fn handle_generate_command(matches: &ArgMatches) -> Result<(), Whatever> {
    let input_file = matches.get_one::<String>("input").unwrap();
    let generator_name = matches.get_one::<String>("generator").unwrap();
    let output_dir = matches.get_one::<String>("output").unwrap();
    let options: Vec<String> = matches
        .get_many::<String>("option")
        .map(|values| values.cloned().collect())
        .unwrap_or_default();

    println!("Generating code from: {}", input_file);
    println!("Using generator: {}", generator_name);
    println!("Output directory: {}", output_dir);

    // Parse the protobuf file
    // let proto_file = ProtoParser::parse_from_file(input_file)?;
    // Create generator context
    let mut context = GeneratorContext::new(PathBuf::from(input_file), Vec::new());

    // Parse options
    for option in options {
        if let Some((key, value)) = option.split_once('=') {
            context = context.with_option(key.to_string(), value.to_string());
        } else {
            eprintln!(
                "Warning: Invalid option format '{}'. Expected 'key=value'",
                option
            );
        }
    }

    // Create registry with built-in generators
    let generator_opt = generators.iter().find(|g| g.name() == generator_name);

    if generator_opt.is_none() {
        eprintln!("Generator not found: {}", generator_name);
        std::process::exit(1);
    }

    let generator = generator_opt.unwrap();

    // Generate code
    let result = generator.generate(&context)?;

    // Show warnings and errors
    for warning in &result.warnings {
        eprintln!("Warning: {}", warning);
    }

    for error in &result.errors {
        eprintln!("Error: {}", error);
    }

    if result.has_errors() {
        eprintln!("Generation failed due to errors above.");
        std::process::exit(1);
    }

    // Write generated files
    let output_path = Path::new(output_dir);
    GenerationResult::write_generated_files(&result, output_path)?;
    println!("\nGeneration completed successfully!");
    println!("Generated {} files", result.files.len());
    Ok(())
}
