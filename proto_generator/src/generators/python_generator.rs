use std::path::PathBuf;

use crate::generator::*;
use crate::generators::helpers::{camel_to_snake_case, run_protoc, to_rust_message_name};
use log::*;
use snafu::{Whatever, prelude::*};

/// Generator for Python code using protoc for protobuf compilation
pub struct PythonGenerator {}

impl PythonGenerator {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate protoc Python code and parse file descriptors
    fn generate_protoc_datatypes(
        &self,
        context: &GeneratorContext,
    ) -> Result<GenerationResult, Whatever> {
        let temp_dir = tempfile::tempdir().whatever_context("Failed to create temp directory")?;

        // Run protoc with Python output
        std::process::Command::new("protoc")
            .arg(format!("--python_out={}", temp_dir.path().display()))
            .arg(context.proto_file.to_str().unwrap())
            .arg(format!(
                "-I={}",
                context.proto_file.parent().unwrap().display()
            ))
            .status()
            .whatever_context("Failed to run protoc")?;

        let mut result = GenerationResult::new();

        // Copy generated files to result
        for entry in
            std::fs::read_dir(&temp_dir).whatever_context("Failed to read temp directory")?
        {
            let entry = entry.whatever_context("Failed to read directory entry")?;
            let path = entry.path();
            if path.is_file() {
                let content =
                    std::fs::read_to_string(&path).whatever_context("Failed to read file")?;
                let file_name = path
                    .file_name()
                    .with_whatever_context(|| "Failed to get filename")?
                    .to_str()
                    .with_whatever_context(|| "Failed to convert filename to string")?
                    .to_string();

                result.add_file(PathBuf::from(file_name), GeneratedFile::new(content));
            }
        }

        Ok(result)
    }

    fn generate_service_code(
        &self,
        context: &GeneratorContext,
    ) -> Result<GenerationResult, Whatever> {
        let descriptor_set = run_protoc(context.proto_file.clone())?;
        let mut result = GenerationResult::new();

        for file in descriptor_set.file {
            let package = file.package.unwrap();
            let mut content = String::new();

            content += &self.generate_common_imports();

            info!(
                r#"File: {file_name}"#,
                file_name = file.name.unwrap_or_default()
            );

            // Generate service base classes
            for service in file.service.clone() {
                content += &self.generate_service_class(&service, &package);
            }
            content += "\n";

            // Generate server classes
            for service in file.service.clone() {
                content += &self.generate_server_class(&service, &package);
            }

            // Generate client classes
            for service in file.service {
                content += &self.generate_client_class(&service, &package);
            }

            result.add_file(
                PathBuf::from(format!(r#"{package}_services.py"#)),
                GeneratedFile::new(content),
            );
        }

        Ok(result)
    }

    fn generate_common_imports(&self) -> String {
        r#"from abc import ABC, abstractmethod
from typing import Optional
from google.protobuf import message
from .database_driver_v1_pb2 import *
from .proto_exception import *

class ProtoError(Exception):
    def __init__(self, error_type: str, details: str):
        self.error_type = error_type
        self.details = details
        super().__init__(f"{error_type}: {details}")

"#
        .to_string()
    }

    fn generate_service_class(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        package: &str,
    ) -> String {
        let service_name = service.name.as_ref().unwrap_or(&String::new()).clone();

        let mut content = format!(
            r#"class {service_name}(ABC):
"#
        );

        for method in &service.method {
            content += &self.generate_service_method(method, package);
        }

        content += "\n";
        content
    }

    fn generate_service_method(
        &self,
        method: &crate::protobuf::MethodDescriptorProto,
        package: &str,
    ) -> String {
        let input_type = to_rust_message_name(
            &package.to_string(),
            &method.input_type.as_ref().unwrap_or(&String::new()).clone(),
        );
        let output_type = to_rust_message_name(
            &package.to_string(),
            &method
                .output_type
                .as_ref()
                .unwrap_or(&String::new())
                .clone(),
        );
        let name = camel_to_snake_case(method.name.as_ref().unwrap_or(&String::new()));

        format!(
            r#"    @abstractmethod
    def {name}(self, request: {input_type}) -> {output_type}:
        pass

"#
        )
    }

    fn generate_server_class(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        package: &str,
    ) -> String {
        let service_name = service.name.as_ref().unwrap_or(&String::new()).clone();

        format!(
            r#"class {service_name}Server({service_name}):
    def handle_message(self, method: str, message_bytes: bytes) -> bytes:
        try:
            # Dispatch to appropriate method
            method_map = {{
{}"#,
            service
                .method
                .iter()
                .map(|m| {
                    let name = camel_to_snake_case(m.name.as_ref().unwrap_or(&String::new()));
                    let input_type = to_rust_message_name(
                        &package.to_string(),
                        &m.input_type.as_ref().unwrap_or(&String::new()).clone(),
                    );
                    format!(r#"                '{name}': (self.{name}, {input_type})"#)
                })
                .collect::<Vec<_>>()
                .join(",\n")
        ) + r#"
            }
            
            if method not in method_map:
                raise ProtoError('Transport', f'Unknown method: {method}')
                
            handler, request_class = method_map[method]
            request = request_class()
            request.ParseFromString(message_bytes)
            response = handler(request)
            return response.SerializeToString()
            
        except Exception as e:
            raise ProtoError('Transport', str(e))

"#
    }

    fn generate_client_class(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        package: &str,
    ) -> String {
        let service_name = service.name.as_ref().unwrap_or(&String::new()).clone();
        let service_error = service
            .options
            .as_ref()
            .and_then(|o| o.service_error.as_ref());

        let mut content = format!(
            r#"class {service_name}Client:
    def __init__(self, transport):
        self._transport = transport

"#
        );

        for method in &service.method {
            let name = camel_to_snake_case(method.name.as_ref().unwrap_or(&String::new()));
            let input_type = to_rust_message_name(
                &package.to_string(),
                &method.input_type.as_ref().unwrap_or(&String::new()).clone(),
            );
            let output_type = to_rust_message_name(
                &package.to_string(),
                &method
                    .output_type
                    .as_ref()
                    .unwrap_or(&String::new())
                    .clone(),
            );
            let method_error = method
                .options
                .as_ref()
                .and_then(|o| o.method_error.as_ref());
            let empty_error = "EmptyError".to_string();
            let error_type = method_error.or(service_error).unwrap_or(&empty_error);

            content += &format!(
                r#"    def {name}(self, request: {input_type}) -> {output_type}:
        (code, response_bytes) = self._transport.handle_message('{service_name}', '{name}', request.SerializeToString())
        if code == 0:
            response = {output_type}()
            response.ParseFromString(response_bytes)
            return response
        elif code == 1:
            error = {error_type}()
            error.ParseFromString(response_bytes)
            raise ProtoApplicationException(error)
        elif code == 2:
            error = str(response_bytes)
            raise ProtoTransportException(response_bytes)
        else:
            raise ProtoTransportException(f"Unknown error code: %s", code)

        response.ParseFromString(self._transport.handle_message('{service_name}', '{name}', request.SerializeToString()))
        return response

"#
            );
        }

        content
    }
}

impl Default for PythonGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator for PythonGenerator {
    fn name(&self) -> &str {
        "python"
    }

    fn target_language(&self) -> GeneratedLanguage {
        GeneratedLanguage::Python
    }

    fn description(&self) -> &str {
        "Generates Python code from protobuf definitions"
    }

    fn generate(&self, context: &GeneratorContext) -> Result<GenerationResult, Whatever> {
        let mut result = self.generate_protoc_datatypes(context)?;
        let service_result = self.generate_service_code(context)?;

        for (path, file) in service_result.files {
            result.add_file(path, file);
        }

        Ok(result)
    }

    fn supported_options(&self) -> Vec<GeneratorOption> {
        Vec::new()
    }
}
