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
            .protoc_executable(&context.protoc_path)
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
        let descriptor_set = run_protoc(context)?;
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
        // `OperationCtx` is threaded through the server trait so each operation
        // can observe cancellation itself. sf_core is the only consumer of this
        // generator (see sf_core/build.rs), so naming its module here is safe.
        r#"
use proto_utils::*;
use prost::Message;
use crate::apis::operation_ctx::OperationCtx;
"#
        .to_string()
    }

    /// True if a method is marked `async_first` in the proto: a slow, cancellable
    /// RPC that receives an [`OperationCtx`] and observes cancellation itself.
    fn is_async_first(method: &crate::protobuf::MethodDescriptorProto) -> bool {
        method
            .options
            .as_ref()
            .and_then(|o| o.async_first)
            .unwrap_or(false)
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

        // Only `async_first` (slow, cancellable) RPCs receive an `OperationCtx`.
        // The proto is the single source of truth: marking an RPC changes this
        // signature, so the corresponding impl must accept the ctx or the build
        // fails, and an unmarked RPC cannot accidentally take one.
        let ctx_param = if Self::is_async_first(method) {
            ", ctx: Option<&OperationCtx>"
        } else {
            ""
        };

        match method_error {
            Some(error) => {
                format!(
                    r#"	fn {name}(&self{ctx_param}, input: {input_type}) -> impl std::future::Future<Output = Result<{output_type}, {error}>> + Send;
"#
                )
            }
            None => {
                format!(
                    r#"	fn {name}(&self{ctx_param}, input: {input_type}) -> impl std::future::Future<Output = {output_type}> + Send;
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
        let service_name = service.name.as_ref().unwrap_or(&String::new()).clone();

        let mut content = format!(
            r#"pub trait {service_name}Server : {service_name} {{
	fn handle_message(&self, ctx: Option<&OperationCtx>, method: &str, message: Vec<u8>) -> impl std::future::Future<Output = Result<Vec<u8>, ProtoError<Vec<u8>>>> + Send where Self: Sync {{ async move {{
		match method {{
"#
        );

        for method in &service.method {
            content += &self.generate_server_method_case(method, package);
        }

        content += r#"			_ => Err(ProtoError::Transport(format!("Unknown method: {}", method))),
		}
	} }
}
"#;

        // Lets the transport tell whether an operation observes cancellation
        // itself. Generated from the same `async_first` marker as the ctx
        // parameter, so the two can never disagree: an unmarked RPC keeps the
        // transport-level race, a marked one is left alone to unwind itself.
        content += &format!(
            r#"
/// True if `method` is an `async_first` RPC, i.e. it takes an [`OperationCtx`]
/// and observes cancellation inside the operation. Callers must NOT wrap such a
/// method in a cancellation race of their own — that would drop it before it
/// could finish cleaning up.
pub fn observes_cancellation(method: &str) -> bool {{
	matches!(method{arms})
}}
"#,
            arms = {
                let marked: Vec<String> = service
                    .method
                    .iter()
                    .filter(|m| Self::is_async_first(m))
                    .map(|m| {
                        format!(
                            "\"{}\"",
                            camel_to_snake_case(m.name.as_ref().unwrap_or(&String::new()))
                        )
                    })
                    .collect();
                if marked.is_empty() {
                    String::new()
                } else {
                    format!(", {}", marked.join(" | "))
                }
            }
        );
        content
    }

    /// Generate a match case for server method handling
    fn generate_server_method_case(
        &self,
        method: &crate::protobuf::MethodDescriptorProto,
        package: &str,
    ) -> String {
        let input_type = to_rust_message_name(
            &package.to_string(),
            &method.input_type.as_ref().unwrap_or(&String::new()).clone(),
        );
        let name = camel_to_snake_case(method.name.as_ref().unwrap_or(&String::new()));

        // Marked RPCs get the ctx and observe cancellation themselves, inside
        // the operation, where they still have the state needed to unwind
        // cleanly. Deliberately no race here: one at this layer would drop the
        // operation before any such cleanup could finish.
        let call_args = if Self::is_async_first(method) {
            "ctx, input"
        } else {
            "input"
        };

        format!(
            r#"			"{name}" => {{
				let input = match {input_type}::decode(&message[..]) {{
					Ok(input) => input,
					Err(e) => return Err(ProtoError::Transport(e.to_string())),
				}};
				let result = Box::pin(self.{name}({call_args})).await;
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
	transport: T,
}}
impl<T: Transport> {service_name}Client<T> {{
	pub fn new(transport: T) -> Self {{
		Self {{ transport }}
	}}
"#
        );

        for method in &service.method {
            content += &self.generate_client_method(method, &service_error, package, &service_name);
        }

        content += "}\n";

        // Cancellable counterparts, in their own impl block so the extra bound
        // lands only on these methods: a client over a plain `Transport` keeps
        // working and simply has no cancellable calls. Generated for
        // `async_first` methods only — the marker already means "slow enough to
        // be worth cancelling", and an RPC that cannot block has nothing to
        // cancel.
        let cancellable: Vec<&crate::protobuf::MethodDescriptorProto> = service
            .method
            .iter()
            .filter(|m| Self::is_async_first(m))
            .collect();
        if !cancellable.is_empty() {
            content += &format!(
                r#"
impl<T: CancellableTransport> {service_name}Client<T> {{
	/// Mint an operation handle to dispatch a cancellable call under. See
	/// [`CancellableTransport::register_operation`].
	pub fn register_operation(&self) -> u64 {{
		self.transport.register_operation()
	}}

	/// Cancel an operation by handle, from any thread. See
	/// [`CancellableTransport::cancel_operation`].
	pub fn cancel_operation(&self, operation: u64) {{
		self.transport.cancel_operation(operation)
	}}

	/// Release a handle that was minted but never dispatched. See
	/// [`CancellableTransport::deregister_operation`].
	pub fn deregister_operation(&self, operation: u64) {{
		self.transport.deregister_operation(operation)
	}}
"#
            );
            for method in cancellable {
                content += &self.generate_client_method_cancellable(
                    method,
                    &service_error,
                    package,
                    &service_name,
                );
            }
            content += "}\n";
        }

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
        let (name, input_type, return_type, decode_tail) =
            self.client_method_shape(method, service_error, package);

        format!(
            r#"
    pub async fn {name}(&self, input: {input_type}) -> {return_type} {{
        let result = self.transport.handle_message("{service_name}", "{name}", input.encode_to_vec()).await;
{decode_tail}    }}
"#
        )
    }

    /// The pieces every client method for `method` shares, whatever it dispatches
    /// through: `(name, input type, return type, response-decoding tail)`.
    ///
    /// Split out so the plain and cancellable variants cannot drift in how they
    /// decode a response or which error type they unwrap.
    fn client_method_shape(
        &self,
        method: &crate::protobuf::MethodDescriptorProto,
        service_error: &Option<String>,
        package: &str,
    ) -> (String, String, String, String) {
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

        // Rendered separately from the signature so the decode logic stays in
        // one place.
        let (return_type, decode_tail) = match &method_error {
            Some(error) => (
                format!("Result<{output_type}, ProtoError<{error}>>"),
                format!(
                    r#"        match result {{
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
"#
                ),
            ),
            None => (
                format!("Result<{output_type}, ProtoError<()>>"),
                format!(
                    r#"        match result {{
            Ok(output) => {{
                let output = {output_type}::decode(&output[..]);
                match output {{
                    Ok(output) => Ok(output),
                    Err(e) => Err(ProtoError::Transport(e.to_string())),
                }}
            }},
            Err(ProtoError::Application(_)) => Err(ProtoError::Transport("Unexpected application error".to_string())),
            Err(ProtoError::Transport(e)) => Err(ProtoError::Transport(e)),
        }}
"#
                ),
            ),
        };

        (name, input_type, return_type, decode_tail)
    }

    /// Generate the cancellable counterpart of a client method: the same call and
    /// the same response decoding, dispatched under a caller-supplied operation
    /// handle.
    ///
    /// Shares [`Self::client_method_shape`] with the plain variant so the decode
    /// logic exists once — a new field or error type changes both together.
    fn generate_client_method_cancellable(
        &self,
        method: &crate::protobuf::MethodDescriptorProto,
        service_error: &Option<String>,
        package: &str,
        service_name: &str,
    ) -> String {
        let (name, input_type, return_type, decode_tail) =
            self.client_method_shape(method, service_error, package);

        format!(
            r#"
    /// Cancellable {name}: dispatched under `operation`, so a concurrent
    /// `cancel_operation(operation)` from any thread reaches the running call.
    pub async fn {name}_cancellable(&self, operation: u64, input: {input_type}) -> {return_type} {{
        let result = self.transport.handle_message_cancellable("{service_name}", "{name}", input.encode_to_vec(), operation).await;
{decode_tail}    }}
"#
        )
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
