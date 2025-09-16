mod breaking_changes_processor;
mod breaking_changes_utils;
mod driver_handlers;
mod feature_parser;
mod step_finder;
mod test_discovery;
mod utils;
mod validator;

use clap::Parser;
use std::path::PathBuf;
use validator::GherkinValidator;

#[derive(Parser)]
#[command(name = "gherkin-validator")]
#[command(about = "Validates Gherkin features against test implementations")]
struct Args {
    /// Path to features directory
    #[arg(short, long, default_value = "tests/definitions")]
    features: PathBuf,

    /// Workspace root path
    #[arg(short, long, default_value = ".")]
    workspace: PathBuf,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Output results as JSON
    #[arg(short, long)]
    json: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let validator = GherkinValidator::new(args.workspace, args.features)?;

    if args.json {
        // JSON output mode - includes Breaking Changes processing
        let enhanced_results = validator.validate_all_with_breaking_changes()?;
        let json_output = serde_json::to_string_pretty(&enhanced_results)?;
        println!("{json_output}");
        return Ok(());
    }

    // Regular text output mode
    let results = validator.validate_all_features()?;
    let orphan_results = validator.find_orphaned_tests()?;

    let mut total_features = 0;
    let mut has_failures = false;

    for result in &results {
        total_features += 1;
        println!("\n📋 Feature: {}", result.feature_file.display());

        for validation in &result.validations {
            if validation.test_file_found {
                // Check if this validation has any issues
                let has_missing_methods = validation
                    .warnings
                    .iter()
                    .any(|w| w.contains("No test method found for scenario"));
                let has_missing_steps = !validation.missing_steps.is_empty();

                if has_missing_methods || has_missing_steps {
                    has_failures = true;
                    println!(
                        "  ❌ {:?}: {} (validation failed)",
                        validation.language,
                        validation.test_file_path.as_ref().unwrap().display()
                    );
                } else {
                    println!(
                        "  ✅ {:?}: {}",
                        validation.language,
                        validation.test_file_path.as_ref().unwrap().display()
                    );
                }

                if !validation.missing_steps.is_empty() {
                    if !validation.missing_steps_by_method.is_empty() {
                        println!("     ⚠️  Missing steps by method:");
                        for method_validation in &validation.missing_steps_by_method {
                            let line_info = if let Some(line_number) = method_validation.line_number
                            {
                                format!(" at line {}", line_number)
                            } else {
                                String::new()
                            };
                            println!(
                                "       In method '{}'{} (scenario: {}):",
                                method_validation.method_name,
                                line_info,
                                method_validation.scenario_name
                            );
                            for step in &method_validation.missing_steps {
                                println!("         - {}", step);
                            }
                        }
                    } else {
                        println!("     ⚠️  Missing steps:");
                        for step in &validation.missing_steps {
                            println!("       - {}", step);
                        }
                    }
                }

                if args.verbose && !validation.implemented_steps.is_empty() {
                    println!("     ✅ Implemented steps:");
                    for step in &validation.implemented_steps {
                        println!("       - {}", step);
                    }
                }
            } else {
                has_failures = true;
                println!("  ❌ {:?}: No test file found", validation.language);
            }

            if !validation.warnings.is_empty() {
                for warning in &validation.warnings {
                    println!("     ⚠️  {}", warning);
                }
            }
        }
    }

    // Check for orphaned tests
    let mut has_orphans = false;
    if !orphan_results.is_empty() {
        has_orphans = true;
        println!("\n🔍 Orphaned Tests Found:");
        for orphan_validation in &orphan_results {
            println!("  {:?}:", orphan_validation.language);

            // Separate orphaned files from files with orphaned methods
            let orphaned_files: Vec<_> = orphan_validation
                .orphaned_files
                .iter()
                .filter(|f| f.orphaned_methods.is_empty())
                .collect();
            let files_with_orphaned_methods: Vec<_> = orphan_validation
                .orphaned_files
                .iter()
                .filter(|f| !f.orphaned_methods.is_empty())
                .collect();

            // Report completely orphaned files
            if !orphaned_files.is_empty() {
                println!("    🗂️  Orphaned files (don't match any feature):");
                for orphaned_file in orphaned_files {
                    println!("      📄 {}", orphaned_file.file_path.display());
                }
            }

            // Report files with orphaned methods
            if !files_with_orphaned_methods.is_empty() {
                println!("    🔧 Files with orphaned methods:");
                for orphaned_file in files_with_orphaned_methods {
                    println!("      📄 {}", orphaned_file.file_path.display());
                    println!("        ⚠️  Orphaned methods:");
                    for method in &orphaned_file.orphaned_methods {
                        println!("          - {}", method);
                    }
                }
            }
        }
    }

    println!("\n📊 Summary:");
    println!("  Features: {}", total_features);
    if has_orphans {
        let total_orphaned_files: usize = orphan_results
            .iter()
            .map(|ov| ov.orphaned_files.len())
            .sum();
        println!("  Orphaned test files: {}", total_orphaned_files);
    }

    if has_failures || has_orphans {
        if has_failures && has_orphans {
            println!("❌ Validation failed - missing tests and orphaned tests found");
        } else if has_failures {
            println!("❌ Validation failed - some tests are missing or incomplete");
        } else {
            println!("⚠️  Validation passed but orphaned tests found");
        }
        std::process::exit(1);
    } else {
        println!("✅ All validations passed");
    }

    Ok(())
}
