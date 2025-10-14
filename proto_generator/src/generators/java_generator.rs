use std::path::PathBuf;

use crate::generators::helpers::{run_protoc, snake_to_pascal_case};
use crate::{generator::*, helpers::camel_to_snake_case};
use log::*;
use snafu::{Whatever, prelude::*};

/// Generator for Java code using protoc for protobuf compilation
pub struct JavaGenerator {}

impl JavaGenerator {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate protoc Java code
    fn generate_protoc_datatypes(
        &self,
        context: &GeneratorContext,
    ) -> Result<GenerationResult, Whatever> {
        let temp_dir = tempfile::tempdir().whatever_context("Failed to create temp directory")?;

        // Run protoc with Java output
        let status = std::process::Command::new("protoc")
            .arg(format!("--java_out={}", temp_dir.path().display()))
            .arg(context.proto_file.to_str().unwrap())
            .arg(format!(
                "-I={}",
                context.proto_file.parent().unwrap().display()
            ))
            .status()
            .whatever_context("Failed to run protoc")?;

        if !status.success() {
            snafu::whatever!("protoc failed with status: {}", status);
        }

        let mut result = GenerationResult::new();

        // Walk the temp directory to find all generated Java files
        for entry in walkdir::WalkDir::new(&temp_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "java") {
                let content =
                    std::fs::read_to_string(path).whatever_context("Failed to read file")?;

                // Get relative path from temp_dir
                let relative_path = path
                    .strip_prefix(&temp_dir)
                    .whatever_context("Failed to strip prefix")?;

                result.add_file(relative_path.to_path_buf(), GeneratedFile::new(content))
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
            // Get the proto file name first
            let proto_file_name = file.name.clone().unwrap_or_else(|| "unknown".to_string());

            info!(r#"File: {file_name}"#, file_name = proto_file_name);

            // Get the outer class name from the proto file name
            let outer_class_name = Self::proto_file_to_outer_class(&proto_file_name);
            let proto_package = file.package.whatever_context("Proto package not found")?;
            let java_package_opt = file.options.as_ref().and_then(|o| o.java_package.as_ref());
            let java_package = java_package_opt.unwrap_or(&proto_package);

            // Generate service code for each service
            for service in file.service.clone() {
                let service_name = service.name.as_ref().unwrap();
                let package_path = java_package.replace('.', "/");

                // Generate service interface
                let interface_content = self.generate_service_interface_file(
                    &service,
                    java_package,
                    &proto_package,
                    &outer_class_name,
                );
                let interface_file_path =
                    PathBuf::from(format!("{}/{}Service.java", package_path, service_name));
                result.add_file(interface_file_path, GeneratedFile::new(interface_content));

                // Generate client class
                let client_content = self.generate_client_class_file(
                    &service,
                    java_package,
                    &proto_package,
                    &outer_class_name,
                );
                let client_file_path = PathBuf::from(format!(
                    "{}/{}ServiceClient.java",
                    package_path, service_name
                ));
                result.add_file(client_file_path, GeneratedFile::new(client_content));
            }
        }

        Ok(result)
    }

    /// Convert proto file name to Java outer class name
    /// e.g., "database_driver_v1.proto" -> "DatabaseDriverV1"
    /// or "protobuf/database_driver_v1.proto" -> "DatabaseDriverV1"
    fn proto_file_to_outer_class(proto_file: &str) -> String {
        // Extract just the filename (remove directory path)
        let filename = proto_file.split('/').next_back().unwrap_or(proto_file);

        // Remove .proto extension
        let name = filename.trim_end_matches(".proto");

        // Convert snake_case to PascalCase
        snake_to_pascal_case(name)
    }

    fn generate_service_interface_file(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        java_package: &str,
        proto_package: &str,
        outer_class: &str,
    ) -> String {
        let service_name = service.name.as_ref().unwrap();
        let mut content = String::new();

        // Package declaration
        content += &format!("package {};\n\n", java_package);

        // Interface
        content += &format!(
            r#"
import com.snowflake.unicore.TransportException;

/**
 * Service interface for {}
 * This file is auto-generated. Do not edit manually.
 */
public interface {}Service {{
"#,
            service_name, service_name
        );

        // Generate method signatures
        for method in &service.method {
            let method_name = Self::to_camel_case(method.name.as_ref().unwrap());
            let input_type = Self::to_java_type_with_outer(
                method.input_type.as_ref().unwrap(),
                proto_package,
                outer_class,
            );
            let output_type = Self::to_java_type_with_outer(
                method.output_type.as_ref().unwrap(),
                proto_package,
                outer_class,
            );

            content += &format!(
                r#"    /**
     * Method: {}
     */
    {} {}({} request) throws ServiceException, TransportException;

"#,
                method_name, output_type, method_name, input_type
            );
        }
        let service_error = service
            .options
            .as_ref()
            .and_then(|o| o.service_error.as_ref());

        let error_type = service_error
            .map(|e| Self::to_java_type_with_outer(e, proto_package, outer_class))
            .expect("Service error not found");
        content += &format!(
            r#"
    class ServiceException extends RuntimeException {{
        public final {error_type} error;
        public ServiceException({error_type} error) {{
            super(error.toString());
            this.error = error;
        }}
    }}
}}
"#
        );
        content
    }

    fn generate_client_class_file(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        java_package: &str,
        proto_package: &str,
        outer_class: &str,
    ) -> String {
        let service_name = service.name.as_ref().unwrap();
        let service_error = service
            .options
            .as_ref()
            .and_then(|o| o.service_error.as_ref());

        let mut content = String::new();

        // Package declaration
        // TODO: Move this string formatting to a template file for better readability
        content += &format!("package {};\n\n", java_package);

        // Import only Message, protobuf classes are now in the same package
        content += &format!(
            r#"import com.google.protobuf.Message;
import com.snowflake.unicore.CoreTransport;
import com.snowflake.unicore.CoreTransport.TransportResponse;
import com.snowflake.unicore.TransportException;
import com.google.protobuf.InvalidProtocolBufferException;
import {java_package}.{service_name}Service;

"#
        );

        content += &format!(
            r#"/**
 * Client implementation for {service_name}
 */
public class {service_name}ServiceClient implements {service_name}Service {{

    private final CoreTransport transport;
    
    public {service_name}ServiceClient(CoreTransport transport) {{
        this.transport = transport;
    }}
    
"#
        );

        // Generate client methods
        for method in &service.method {
            let method_name = Self::to_camel_case(method.name.as_ref().unwrap());
            let input_type = Self::to_java_type_with_outer(
                method.input_type.as_ref().unwrap(),
                proto_package,
                outer_class,
            );
            let output_type = Self::to_java_type_with_outer(
                method.output_type.as_ref().unwrap(),
                proto_package,
                outer_class,
            );
            let method_error = method
                .options
                .as_ref()
                .and_then(|o| o.method_error.as_ref());
            let error_type = method_error
                .or(service_error)
                .map(|e| Self::to_java_type_with_outer(e, proto_package, outer_class))
                .unwrap_or_else(|| "Exception".to_string());
            let proto_method_name = camel_to_snake_case(method.name.as_ref().unwrap());

            content += &format!(
                r#"    /**
     * Method: {method_name}
     */
    public {output_type} {method_name}({input_type} request) throws ServiceException, TransportException {{
        TransportResponse response = transport.handleMessage(
            "{service_name}",
            "{proto_method_name}",
            request.toByteArray()
        );
        
        int code = response.getCode();
        byte[] responseBytes = response.getResponseBytes();
        
        if (code == CoreTransport.CODE_SUCCESS) {{
            try {{
                return {output_type}.parseFrom(responseBytes);
            }} catch (InvalidProtocolBufferException e) {{
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }}
        }} else if (code == CoreTransport.CODE_APPLICATION_ERROR) {{
            try {{
                {error_type} error = {error_type}.parseFrom(responseBytes);
                throw new ServiceException(error);
            }} catch (InvalidProtocolBufferException e) {{
                throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
            }}
        }} else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {{
            String errorMessage = new String(responseBytes);
            throw new TransportException(errorMessage);
        }} else {{
            throw new TransportException("Unknown error code: " + code);
        }}
    }}
    
"#
            );
        }

        content += "}\n";
        content
    }

    /// Convert protobuf method name to Java camelCase
    fn to_camel_case(name: &str) -> String {
        let mut chars = name.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        }
    }

    fn lstrip_package<'a>(proto_type: &'a str, package: &str) -> &'a str {
        proto_type
            .strip_prefix(".")
            .unwrap_or(proto_type)
            .strip_prefix(package)
            .unwrap_or(proto_type)
            .strip_prefix(".")
            .unwrap_or(proto_type)
    }

    /// Convert protobuf type name to Java type name with outer class prefix
    /// e.g., ".database_driver_v1.DatabaseNewRequest" -> "DatabaseDriverV1.DatabaseNewRequest"
    fn to_java_type_with_outer(proto_type: &str, package: &str, outer_class: &str) -> String {
        format!(
            "{}.{}",
            outer_class,
            Self::lstrip_package(proto_type, package)
        )
    }
}

impl Default for JavaGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator for JavaGenerator {
    fn name(&self) -> &str {
        "java"
    }

    fn target_language(&self) -> GeneratedLanguage {
        GeneratedLanguage::Java
    }

    fn description(&self) -> &str {
        "Generates Java code from protobuf definitions"
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
