use std::path::PathBuf;

use crate::generator::*;
use crate::generators::helpers::{camel_to_snake_case, run_protoc, to_rust_message_name};
use log::*;
use snafu::{Whatever, prelude::*};

/// Generator for Rust code using prost_build for protobuf compilation
#[derive(Default)]
pub struct RustGenerator {}

impl RustGenerator {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate prost code and parse file descriptors
    fn generate_prost_datatypes(
        &self,
        context: &GeneratorContext,
    ) -> Result<GenerationResult, Whatever> {
        let temp_dir = tempfile::tempdir().whatever_context("Failed to create temp directory")?;

        prost_build::Config::new()
            .out_dir(temp_dir.path().to_path_buf())
            .compile_protos(
                &[&context.proto_file],
                &[context.proto_file.parent().unwrap()],
            )
            .whatever_context("Failed to compile protos")?;

        let mut result = GenerationResult::new();

        // List all files in temp_dir
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

            // Generate service traits
            for service in file.service.clone() {
                content += &self.generate_service_trait(&service, &package);
            }
            content += "\n";

            // Generate server traits
            for service in file.service.clone() {
                content += &self.generate_server_trait(&service, &package);
            }

            // Generate client structs
            for service in file.service {
                content += &self.generate_client_struct(&service, &package);
            }

            result.add_file(
                PathBuf::from(format!(r#"{package}.rs"#)),
                GeneratedFile::new(content),
            );
        }

        Ok(result)
    }

    /// Generate common types (ProtoError and Transport trait)
    fn generate_common_imports(&self) -> String {
        r#"
use proto_utils::*;
use prost::Message;
"#
        .to_string()
    }

    /// Generate a service trait
    fn generate_service_trait(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        package: &str,
    ) -> String {
        let service_error = service
            .options
            .as_ref()
            .unwrap_or(&Default::default())
            .service_error
            .clone();
        let service_name = service.name.as_ref().unwrap_or(&String::new()).clone();

        let mut content = format!(
            r#"pub trait {service_name} {{
"#
        );

        for method in &service.method {
            content += &self.generate_service_method(method, &service_error, package);
        }

        content += "}\n";
        content
    }

    /// Generate a single service method signature
    fn generate_service_method(
        &self,
        method: &crate::protobuf::MethodDescriptorProto,
        service_error: &Option<String>,
        package: &str,
    ) -> String {
        let method_error = method
            .options
            .as_ref()
            .unwrap_or(&Default::default())
            .method_error
            .clone()
            .or_else(|| service_error.clone());

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

        match method_error {
            Some(error) => {
                format!(
                    r#"	fn {name}(input: {input_type}) -> Result<{output_type}, {error}>;
"#
                )
            }
            None => {
                format!(
                    r#"	fn {name}(input: {input_type}) -> {output_type};
"#
                )
            }
        }
    }

    /// Generate a server trait
    fn generate_server_trait(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        package: &str,
    ) -> String {
        let service_error = service
            .options
            .as_ref()
            .unwrap_or(&Default::default())
            .service_error
            .clone();
        let service_name = service.name.as_ref().unwrap_or(&String::new()).clone();

        let mut content = format!(
            r#"pub trait {service_name}Server : {service_name} {{
	fn handle_message(method: &str, message: Vec<u8>) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {{
		match method {{
"#
        );

        for method in &service.method {
            content += &self.generate_server_method_case(method, &service_error, package);
        }

        content += r#"			_ => Err(ProtoError::Transport(format!("Unknown method: {}", method))),
		}
	}
}
"#;
        content
    }

    /// Generate a match case for server method handling
    fn generate_server_method_case(
        &self,
        method: &crate::protobuf::MethodDescriptorProto,
        _service_error: &Option<String>,
        package: &str,
    ) -> String {
        let input_type = to_rust_message_name(
            &package.to_string(),
            &method.input_type.as_ref().unwrap_or(&String::new()).clone(),
        );
        let name = camel_to_snake_case(method.name.as_ref().unwrap_or(&String::new()));

        format!(
            r#"			"{name}" => {{
				let input = match {input_type}::decode(&message[..]) {{
					Ok(input) => input,
					Err(e) => return Err(ProtoError::Transport(e.to_string())),
				}};
				let result = Self::{name}(input);
				match result {{
				Ok(output) => Ok(output.encode_to_vec()),
				Err(e) => Err(ProtoError::Application(e.encode_to_vec())),
				}}
			}}
"#
        )
    }

    /// Generate a client struct with methods
    fn generate_client_struct(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        package: &str,
    ) -> String {
        let service_name = service.name.as_ref().unwrap_or(&String::new()).clone();
        let service_error = service
            .options
            .as_ref()
            .unwrap_or(&Default::default())
            .service_error
            .clone();

        let mut content = format!(
            r#"pub struct {service_name}Client<T: Transport> {{
	_marker: ::core::marker::PhantomData<T>,
}}
impl<T: Transport> {service_name}Client<T> {{
"#
        );

        for method in &service.method {
            content += &self.generate_client_method(method, &service_error, package, &service_name);
        }

        content += "}\n";
        content
    }

    /// Generate a client method implementation
    fn generate_client_method(
        &self,
        method: &crate::protobuf::MethodDescriptorProto,
        service_error: &Option<String>,
        package: &str,
        service_name: &str,
    ) -> String {
        let method_error = method
            .options
            .as_ref()
            .unwrap_or(&Default::default())
            .method_error
            .clone()
            .or_else(|| service_error.clone());

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

        match method_error {
            Some(error) => {
                format!(
                    r#"
    pub fn {name}(input: {input_type}) -> Result<{output_type}, ProtoError<{error}>> {{
        let result = T::handle_message("{service_name}", "{name}", input.encode_to_vec());
        match result {{
            Ok(output) => {{
                let output = {output_type}::decode(&output[..]);
                match output {{
                    Ok(output) => Ok(output),
                    Err(e) => Err(ProtoError::Transport(e.to_string())),
                }}
            }},
            Err(ProtoError::Application(e)) => {{
                let output = {error}::decode(&e[..]);
                match output {{
                    Ok(output) => Err(ProtoError::Application(output)),
                    Err(e) => Err(ProtoError::Transport(e.to_string())),
                }}
            }},
            Err(ProtoError::Transport(e)) => Err(ProtoError::Transport(e)),
        }}
    }}
"#
                )
            }
            None => {
                format!(
                    r#"	fn {name}(input: {input_type}) -> {output_type};
"#
                )
            }
        }
    }
}

impl CodeGenerator for RustGenerator {
    fn name(&self) -> &str {
        "rust"
    }

    fn target_language(&self) -> GeneratedLanguage {
        GeneratedLanguage::Rust
    }

    fn description(&self) -> &str {
        "Generates Rust code using prost_build for protobuf compilation during build time"
    }

    fn generate(&self, context: &GeneratorContext) -> Result<GenerationResult, Whatever> {
        let mut result = GenerationResult::new();
        let datatypes = self.generate_prost_datatypes(context)?;
        let services = self.generate_service_code(context)?;
        result.merge(datatypes);
        result.merge(services);
        Ok(result)
    }

    fn supported_options(&self) -> Vec<GeneratorOption> {
        Vec::new()
    }
}
