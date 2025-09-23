use std::path::PathBuf;

use crate::generator::*;
use crate::generators::helpers::run_protoc;
use snafu::{Whatever, prelude::*};

/// Generator for JSON descriptor files from protobuf definitions
pub struct JsonGenerator;

impl JsonGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator for JsonGenerator {
    fn name(&self) -> &str {
        "json"
    }

    fn target_language(&self) -> GeneratedLanguage {
        GeneratedLanguage::Other("json".to_string())
    }

    fn description(&self) -> &str {
        "Generates JSON descriptor files from protobuf definitions"
    }

    fn generate(&self, context: &GeneratorContext) -> Result<GenerationResult, Whatever> {
        // Run protoc to get the descriptor set
        let descriptor_set = run_protoc(context.proto_file.clone())?;

        // Write the JSON descriptor
        let mut result = GenerationResult::new();

        let json = serde_json::to_string_pretty(&descriptor_set)
            .whatever_context("Failed to serialize to JSON")?;

        let filename = context
            .proto_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let filename = filename.replace(".proto", ".json");
        result.add_file(PathBuf::from(filename), GeneratedFile::new(json));
        // Return empty result since we only write to files, don't generate content
        Ok(result)
    }

    fn supported_options(&self) -> Vec<GeneratorOption> {
        Vec::new()
    }
}
