use snafu::{Whatever, prelude::*};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Context information passed to generators
#[derive(Debug, Clone)]
pub struct GeneratorContext {
    pub proto_file: PathBuf,
    pub include_directories: Vec<PathBuf>,
    pub options: HashMap<String, String>,
}

impl GeneratorContext {
    pub fn new(proto_file: PathBuf, include_directories: Vec<PathBuf>) -> Self {
        Self {
            proto_file,
            include_directories,
            options: HashMap::new(),
        }
    }

    pub fn with_option(mut self, key: String, value: String) -> Self {
        self.options.insert(key, value);
        self
    }

    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.options.get(key)
    }
}

/// Generated file information
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub content: String,
}

impl GeneratedFile {
    pub fn new(content: String) -> Self {
        Self { content }
    }

    pub const EMPTY: Self = Self {
        content: String::new(),
    };

    pub fn merged(&self, other: &GeneratedFile) -> Self {
        Self {
            content: self.content.clone() + &other.content,
        }
    }
}

/// Supported target languages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratedLanguage {
    Rust,
    Go,
    Python,
    TypeScript,
    Java,
    CSharp,
    Cpp,
    Other(String),
}

impl GeneratedLanguage {
    pub fn file_extension(&self) -> &str {
        match self {
            GeneratedLanguage::Rust => ".rs",
            GeneratedLanguage::Go => ".go",
            GeneratedLanguage::Python => ".py",
            GeneratedLanguage::TypeScript => ".ts",
            GeneratedLanguage::Java => ".java",
            GeneratedLanguage::CSharp => ".cs",
            GeneratedLanguage::Cpp => ".cpp",
            GeneratedLanguage::Other(_) => ".txt",
        }
    }
}

/// Result of code generation
#[derive(Debug)]
pub struct GenerationResult {
    pub files: HashMap<PathBuf, GeneratedFile>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl GenerationResult {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_file(&mut self, path: PathBuf, file: GeneratedFile) {
        self.files.insert(path, file);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn merge(&mut self, other: GenerationResult) {
        let mut new_files = HashMap::new();
        let key_set = other
            .files
            .keys()
            .chain(self.files.keys())
            .collect::<HashSet<_>>();
        for path in key_set {
            let empty = GeneratedFile::EMPTY;
            let self_file = self.files.get(path).unwrap_or(&empty);
            let other_file = other.files.get(path).unwrap_or(&empty);
            let merged_file = self_file.merged(other_file);
            new_files.insert(path.clone(), merged_file);
        }
        self.files = new_files;
        self.warnings.extend(other.warnings);
        self.errors.extend(other.errors);
    }

    pub fn write_generated_files(&self, base_path: &Path) -> Result<(), Whatever> {
        for (path, file) in &self.files {
            let full_path = base_path.join(path);
            // Create parent directories if they don't exist
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)
                    .whatever_context("Failed to create parent directories")?;
            }
            std::fs::write(&full_path, &file.content).whatever_context("Failed to write file")?;
        }
        Ok(())
    }
}

impl Default for GenerationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Main trait for pluggable code generators
pub trait CodeGenerator {
    /// Name of the generator (e.g., "rust-grpc", "go-grpc", "typescript-client")
    fn name(&self) -> &str;

    /// Target language this generator produces
    fn target_language(&self) -> GeneratedLanguage;

    /// Description of what this generator does
    fn description(&self) -> &str;

    /// Generate code from the protobuf AST
    fn generate(&self, context: &GeneratorContext) -> Result<GenerationResult, Whatever>;

    /// Get generator-specific options and their descriptions
    fn supported_options(&self) -> Vec<GeneratorOption> {
        Vec::new()
    }
}

/// Description of a generator option
#[derive(Debug, Clone)]
pub struct GeneratorOption {
    pub name: String,
    pub description: String,
    pub default_value: Option<String>,
    pub required: bool,
}

impl GeneratorOption {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            default_value: None,
            required: false,
        }
    }

    pub fn with_default(mut self, default: String) -> Self {
        self.default_value = Some(default);
        self
    }

    /// Convert snake_case to PascalCase
    pub fn snake_to_pascal_case(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect()
    }

    /// Convert snake_case to camelCase
    pub fn snake_to_camel_case(s: &str) -> String {
        let pascal = Self::snake_to_pascal_case(s);
        let mut chars = pascal.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        }
    }

    /// Generate package/namespace declaration for different languages
    pub fn generate_package_declaration(package: &str, language: &GeneratedLanguage) -> String {
        match language {
            GeneratedLanguage::Rust => format!("// Package: {}\n", package),
            GeneratedLanguage::Go => {
                format!(
                    "package {}\n",
                    package.split('.').next_back().unwrap_or(package)
                )
            }
            GeneratedLanguage::TypeScript => format!("// Package: {}\n", package),
            GeneratedLanguage::Python => format!("# Package: {}\n", package),
            GeneratedLanguage::Java => format!("package {};\n", package),
            GeneratedLanguage::CSharp => format!("namespace {}\n{{\n", package),
            _ => format!("// Package: {}\n", package),
        }
    }

    /// Write generated files to disk
    pub fn write_generated_files(
        result: &GenerationResult,
        base_path: &Path,
    ) -> Result<(), Whatever> {
        for (path, file) in &result.files {
            println!("Writing file: {}", path.to_string_lossy());
            let file_path = base_path.join(path);

            // Create directory if it doesn't exist
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)
                    .whatever_context("Failed to create parent directories")?;
            }
            std::fs::write(&file_path, &file.content).whatever_context("Failed to write file")?;
            println!("Generated: {}", file_path.display());
        }

        Ok(())
    }
}
