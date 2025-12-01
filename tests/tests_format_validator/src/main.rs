mod behavior_differences_processor;
mod behavior_differences_utils;
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
        // JSON output mode - includes Behavior Differences processing
        let enhanced_results = validator.validate_all_with_breaking_changes()?;
        let json_output = serde_json::to_string_pretty(&enhanced_results)?;
        println!("{json_output}");
        return Ok(());
    }

    // Regular text output mode
    let results = validator.validate_all_features()?;
    let orphan_results = validator.find_orphaned_tests()?;
    let untagged_features = validator.find_untagged_features()?;

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
                        println!("     ⚠️  Issues:");
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

                // Show validation errors even when no test file
                if !validation.missing_steps.is_empty() {
                    println!("     ⚠️  Issues:");
                    for step in &validation.missing_steps {
                        println!("       - {}", step);
                    }
                }
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
        println!("\n❌ VALIDATION ERROR - Orphaned Tests Found:");
        println!("   Tests exist that are not referenced in any feature file.");
        println!("   Either add them to a feature file or remove them.\n");
        for orphan_validation in &orphan_results {
            // Group by reason
            use crate::validator::OrphanReason;

            let no_matching_feature: Vec<_> = orphan_validation
                .orphaned_files
                .iter()
                .filter(|f| matches!(f.reason, OrphanReason::NoMatchingFeature))
                .collect();

            let language_not_needed: Vec<_> = orphan_validation
                .orphaned_files
                .iter()
                .filter(|f| matches!(f.reason, OrphanReason::LanguageMarkedAsNotNeeded))
                .collect();

            let missing_generic_tag: Vec<_> = orphan_validation
                .orphaned_files
                .iter()
                .filter(|f| matches!(f.reason, OrphanReason::FeatureMissingGenericLanguageTag))
                .collect();

            let no_scenario_tags: Vec<_> = orphan_validation
                .orphaned_files
                .iter()
                .filter(|f| matches!(f.reason, OrphanReason::FeatureExistsButNoScenarioTags))
                .collect();

            let orphaned_methods: Vec<_> = orphan_validation
                .orphaned_files
                .iter()
                .filter(|f| matches!(f.reason, OrphanReason::MethodsWithoutScenarioTags))
                .collect();

            // Only print language header if there are actually orphaned files to report
            if no_matching_feature.is_empty()
                && language_not_needed.is_empty()
                && missing_generic_tag.is_empty()
                && no_scenario_tags.is_empty()
                && orphaned_methods.is_empty()
            {
                continue;
            }

            println!("  {:?}:", orphan_validation.language);

            use crate::test_discovery::Language as Lang;
            let language_tag = match orphan_validation.language {
                Lang::Rust => "core",
                Lang::Jdbc => "jdbc",
                Lang::Odbc => "odbc",
                Lang::Python => "python",
                _ => "language",
            };

            // Report files with no matching feature
            if !no_matching_feature.is_empty() {
                println!("    🗂️  No matching feature file:");
                for orphaned_file in no_matching_feature {
                    println!("      ❌ {}", orphaned_file.file_path.display());
                }
            }

            // Report files where language is explicitly marked as not needed
            if !language_not_needed.is_empty() {
                println!("    🗂️  Test file exists but language marked as not needed:");
                for orphaned_file in language_not_needed {
                    println!("      ❌ {}", orphaned_file.file_path.display());
                    println!(
                        "         → Feature has @{}_not_needed tag. Remove test file or remove exclusion tag.",
                        language_tag
                    );
                }
            }

            // Report files where scenarios have tags but feature is missing generic language tag
            if !missing_generic_tag.is_empty() {
                println!("    🗂️  Feature missing generic language tag:");
                for orphaned_file in missing_generic_tag {
                    println!("      ❌ {}", orphaned_file.file_path.display());
                    println!(
                        "         → Add feature-level tag @{} (scenarios have @{}_e2e/@{}_int)",
                        language_tag, language_tag, language_tag
                    );
                }
            }

            // Report files where feature exists but has no scenario tags for this language
            if !no_scenario_tags.is_empty() {
                println!("    🗂️  Feature exists but no scenarios have language/level tags:");
                for orphaned_file in no_scenario_tags {
                    println!("      ❌ {}", orphaned_file.file_path.display());
                    println!(
                        "         → Add scenario tags like @{}_e2e or @{}_int",
                        language_tag, language_tag
                    );
                }
            }

            // Report files with orphaned methods
            if !orphaned_methods.is_empty() {
                println!("    🔧 Orphaned methods (scenarios missing language tags):");
                for orphaned_file in orphaned_methods {
                    println!("      ❌ {}", orphaned_file.file_path.display());
                    println!("        Methods:");
                    for method in &orphaned_file.orphaned_methods {
                        println!("          - {}", method);
                    }
                }
            }
        }
    }

    // Display TODO section for untagged features
    if !untagged_features.is_empty() {
        println!("\n📝 TODO - Features without tags:");
        println!("   These features have no tags at all and need to be tagged:");
        for feature_path in &untagged_features {
            let feature_name = feature_path.file_stem().unwrap().to_str().unwrap();
            println!("      🔖 {}", feature_name);
        }
        println!(
            "   → Add feature-level tags (e.g., @core @python) and scenario-level tags (e.g., @core_e2e @python_e2e)"
        );
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
    if !untagged_features.is_empty() {
        println!("  Untagged features (TODO): {}", untagged_features.len());
    }

    if has_failures || has_orphans {
        if has_failures && has_orphans {
            println!("❌ VALIDATION FAILED - missing implementations and orphaned tests");
        } else if has_failures {
            println!("❌ VALIDATION FAILED - some tests are missing or incomplete");
        } else {
            println!(
                "❌ VALIDATION FAILED - orphaned tests found (tests without feature definitions)"
            );
        }
        std::process::exit(1);
    } else {
        println!("✅ All validations passed");
    }

    Ok(())
}
