use std::path::PathBuf;

use crate::generator::*;
use crate::generators::helpers::{camel_to_snake_case, run_protoc, snake_to_pascal_case};
use log::*;
use snafu::{Whatever, prelude::*};

/// Generator for dotnet code using protoc for protobuf compilation
pub struct CSharpGenerator {}

impl CSharpGenerator {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate protoc C# code (message types, enums)
    fn generate_protoc_datatypes(
        &self,
        context: &GeneratorContext,
    ) -> Result<GenerationResult, Whatever> {
        let temp_dir = tempfile::tempdir().whatever_context("Failed to create temp directory")?;

        let out_dir = temp_dir.path().display();
        let proto_file = context.proto_file.to_str().unwrap();
        let include_dir = context.proto_file.parent().unwrap().display();

        // Run protoc with C# output
        let output = std::process::Command::new(&context.protoc_path)
            .arg(format!("--csharp_out={}", out_dir))
            .arg("--csharp_opt=base_namespace=Snowflake.Data.Proto,internal_access")
            .arg(proto_file)
            .arg(format!("-I={include_dir}"))
            .output()
            .whatever_context("Failed to run protoc --csharp_out")?;

        if !output.status.success() {
            snafu::whatever!(
                "protoc --csharp_out failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let mut result = GenerationResult::new();

        // Walk the temp directory to find all generated C# files
        for entry in walkdir::WalkDir::new(&temp_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "cs") {
                let content =
                    std::fs::read_to_string(path).whatever_context("Failed to read file")?;

                let relative_path = path
                    .strip_prefix(&temp_dir)
                    .whatever_context("Failed to strip prefix")?;

                result.add_file(relative_path.to_path_buf(), GeneratedFile::new(content));
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

        // Generate transport abstraction
        let transport_content = Self::generate_transport_interface();
        result.add_file(
            PathBuf::from("ICoreTransport.cs"),
            GeneratedFile::new(transport_content),
        );

        // Generate TransportResponse
        let response_content = Self::generate_transport_response();
        result.add_file(
            PathBuf::from("TransportResponse.cs"),
            GeneratedFile::new(response_content),
        );

        // Generate exceptions
        let service_exception_content = Self::generate_service_exception();
        result.add_file(
            PathBuf::from("Exceptions/ServiceException.cs"),
            GeneratedFile::new(service_exception_content),
        );

        // Generate error parser
        let general_error_parser = Self::generate_general_error_parser();
        result.add_file(
            PathBuf::from("GeneralErrorParser.cs"),
            GeneratedFile::new(general_error_parser),
        );

        let transport_exception_content = Self::generate_transport_exception();
        result.add_file(
            PathBuf::from("Exceptions/TransportException.cs"),
            GeneratedFile::new(transport_exception_content),
        );

        for file in descriptor_set.file {
            let proto_file_name = file.name.clone().unwrap_or_else(|| "unknown".to_string());
            info!(r#"File: {file_name}"#, file_name = proto_file_name);

            let outer_class_name = Self::proto_file_to_outer_class(&proto_file_name);
            let proto_package = file.package.whatever_context("Proto package not found")?;

            for service in file.service.clone() {
                let service_name = service.name.as_ref().unwrap();

                // Generate service interface
                let interface_content =
                    self.generate_service_interface(&service, &proto_package, &outer_class_name);
                result.add_file(
                    PathBuf::from(format!("{service_name}/I{service_name}Service.cs")),
                    GeneratedFile::new(interface_content),
                );

                // Generate client class
                let client_content =
                    self.generate_client_class(&service, &proto_package, &outer_class_name);
                result.add_file(
                    PathBuf::from(format!("{service_name}/ServiceClient.cs")),
                    GeneratedFile::new(client_content),
                );
            }
        }

        Ok(result)
    }

    fn generate_transport_interface() -> String {
        r#"// <auto-generated>
//   This file was generated by proto_generator. Do not edit manually.
// </auto-generated>

using System.Threading;
using System.Threading.Tasks;
namespace Snowflake.Data.Proto;

/// <summary>
/// Transport abstraction for communicating with sf_core via protobuf-over-FFI.
/// </summary>
internal interface ICoreTransport
{
    /// <summary>
    /// Send a synchronous message to the core and receive a response.
    /// </summary>
    TransportResponse HandleMessage(string service, string method, byte[] request);

    /// <summary>
    /// Send an asynchronous message to the core and receive a response.
    /// </summary>
    Task<TransportResponse> HandleMessageAsync(string service, string method, byte[] request, CancellationToken cancellationToken);
}
"#
        .to_string()
    }

    fn generate_general_error_parser() -> String {
        r#"// <auto-generated>
//   This file was generated by proto_generator. Do not edit manually.
// </auto-generated>

using System;
using System.Text;

namespace Snowflake.Data.Proto;

/// <summary>
/// Abstraction over .net framework insufficient support for spans
/// </summary>
internal static class GeneralErrorParser
{
    public static string GetString(ReadOnlySpan<byte> span)
    {
#if NETFRAMEWORK
        return Encoding.UTF8.GetString(span.ToArray());
#else
        return Encoding.UTF8.GetString(span);
#endif
    }
}
"#
        .to_string()
    }

    fn generate_transport_response() -> String {
        r#"// <auto-generated>
//   This file was generated by proto_generator. Do not edit manually.
// </auto-generated>

using System;
using System.Buffers;

namespace Snowflake.Data.Proto;

/// <summary>
/// Response from the core transport layer.
/// </summary>
/// <param name="Code">Response code: 0 = success, 1 = application error, 2 = transport error.</param>
/// <param name="ResponseBytes">Serialized protobuf response or error bytes.</param>
/// <param name="Buffer">Buffer in which <c>ResponseBytes</c> are defined.</param>
internal record struct TransportResponse(int Code, ArraySegment<byte> ResponseBytes, byte[] Buffer) : IDisposable
{
    private int _disposed = 0;

    /// <summary>Success response code.</summary>
    public const int CodeSuccess = 0;

    /// <summary>Application-level error (deserialize as DriverException).</summary>
    public const int CodeApplicationError = 1;

    /// <summary>Transport-level error (UTF-8 error string).</summary>
    public const int CodeTransportError = 2;

    public void Dispose()
    {
        if (Interlocked.CompareExchange(ref _disposed, 1, 0) != 0)
            return;

        if (Buffer != null)
            ArrayPool<byte>.Shared.Return(Buffer);
    }
}
"#
        .to_string()
    }

    fn generate_service_exception() -> String {
        r#"// <auto-generated>
//   This file was generated by proto_generator. Do not edit manually.
// </auto-generated>

namespace Snowflake.Data.Proto;

/// <summary>
/// Exception thrown when the core returns an application-level error.
/// </summary>
internal sealed class ServiceException : Exception
{
    /// <summary>
    /// The deserialized driver exception from the core.
    /// </summary>
    public DriverException Error { get; }

    public ServiceException(DriverException error)
        : base(error.ToString())
    {
        Error = error;
    }
}
"#
        .to_string()
    }

    fn generate_transport_exception() -> String {
        r#"// <auto-generated>
//   This file was generated by proto_generator. Do not edit manually.
// </auto-generated>

namespace Snowflake.Data.Proto;

/// <summary>
/// Exception thrown when communication with the core fails at the transport level.
/// </summary>
internal sealed class TransportException : Exception
{
    public TransportException(string message)
        : base(message) { }

    public TransportException(string message, Exception innerException)
        : base(message, innerException) { }
}
"#
        .to_string()
    }

    fn generate_service_interface(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        proto_package: &str,
        outer_class: &str,
    ) -> String {
        let service_name = service.name.as_ref().unwrap();

        let mut content = format!(
            r#"// <auto-generated>
//   This file was generated by proto_generator. Do not edit manually.
// </auto-generated>

using System.Threading;
using System.Threading.Tasks;
namespace Snowflake.Data.Proto;

/// <summary>
/// Service interface for {service_name}.
/// </summary>
internal interface I{service_name}Service
{{
"#
        );

        for method in &service.method {
            // Proto method names are already PascalCase (e.g., DatabaseNew)
            let method_name = method.name.as_ref().unwrap().clone();
            let input_type = Self::to_csharp_type(
                method.input_type.as_ref().unwrap(),
                proto_package,
                outer_class,
            );
            let output_type = Self::to_csharp_type(
                method.output_type.as_ref().unwrap(),
                proto_package,
                outer_class,
            );

            // Sync method
            content += &format!("    {output_type} {method_name}({input_type} request);\n\n");

            // Async method
            content += &format!(
                "    Task<{output_type}> {method_name}Async(\n        {input_type} request,\n        CancellationToken cancellationToken);\n\n"
            );
        }

        content += "}\n";
        content
    }

    fn generate_client_class(
        &self,
        service: &crate::protobuf::ServiceDescriptorProto,
        proto_package: &str,
        outer_class: &str,
    ) -> String {
        let service_name = service.name.as_ref().unwrap();
        let service_error = service
            .options
            .as_ref()
            .and_then(|o| o.service_error.as_ref());

        let mut content = format!(
            r#"// <auto-generated>
//   This file was generated by proto_generator. Do not edit manually.
// </auto-generated>

using System.Threading;
using System.Threading.Tasks;
using Google.Protobuf;

namespace Snowflake.Data.Proto;

/// <summary>
/// Client implementation for {service_name}.
/// Dispatches protobuf-serialized requests via <see cref="ICoreTransport"/>.
/// </summary>
internal sealed class {service_name}ServiceClient : I{service_name}Service
{{
    private readonly ICoreTransport _transport;

    public {service_name}ServiceClient(ICoreTransport transport)
    {{
        _transport = transport;
    }}

"#
        );

        for method in &service.method {
            // Proto method names are already PascalCase (e.g., DatabaseNew)
            let method_name = method.name.as_ref().unwrap().clone();
            let proto_method_name = camel_to_snake_case(method.name.as_ref().unwrap());
            let input_type = Self::to_csharp_type(
                method.input_type.as_ref().unwrap(),
                proto_package,
                outer_class,
            );
            let output_type = Self::to_csharp_type(
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
                .map(|e| Self::to_csharp_type(e, proto_package, outer_class))
                .unwrap_or_else(|| "DriverException".to_string());

            // Sync method
            content += &format!(
                r#"    /// <inheritdoc />
    public {output_type} {method_name}({input_type} request)
    {{
        using var response = _transport.HandleMessage("{service_name}", "{proto_method_name}", request.ToByteArray());
        return response.Code switch
        {{
            TransportResponse.CodeSuccess =>
                {output_type}.Parser.ParseFrom(response.ResponseBytes),
            TransportResponse.CodeApplicationError =>
                throw new ServiceException(
                    {error_type}.Parser.ParseFrom(response.ResponseBytes)),
            _ => throw new TransportException(
                GeneralErrorParser.GetString(response.ResponseBytes)),
        }};
    }}

"#
            );

            // Async method
            content += &format!(
                r#"    /// <inheritdoc />
    public async Task<{output_type}> {method_name}Async(
        {input_type} request,
        CancellationToken cancellationToken = default)
    {{
        using var response = await _transport.HandleMessageAsync("{service_name}", "{proto_method_name}", request.ToByteArray(), cancellationToken).ConfigureAwait(false);
        return response.Code switch
        {{
            TransportResponse.CodeSuccess =>
                {output_type}.Parser.ParseFrom(response.ResponseBytes),
            TransportResponse.CodeApplicationError =>
                throw new ServiceException(
                    {error_type}.Parser.ParseFrom(response.ResponseBytes)),
            _ => throw new TransportException(
                GeneralErrorParser.GetString(response.ResponseBytes)),
        }};
    }}

"#
            );
        }

        content += "}\n";
        content
    }

    /// Convert proto file name to C# outer class name
    /// e.g., "database_driver_v1.proto" -> "DatabaseDriverV1"
    fn proto_file_to_outer_class(proto_file: &str) -> String {
        let filename = proto_file.split('/').next_back().unwrap_or(proto_file);
        let name = filename.trim_end_matches(".proto");
        snake_to_pascal_case(name)
    }

    /// Strip the proto package prefix and return the bare message name.
    /// In C# protoc output, messages are top-level classes in the namespace
    /// e.g., ".database_driver_v1.DatabaseNewRequest" -> "DatabaseNewRequest"
    fn to_csharp_type(proto_type: &str, _package: &str, _outer_class: &str) -> String {
        let offset = proto_type.rfind('.').map_or(0, |x| x + 1);
        proto_type[offset..].to_string()
    }
}

impl Default for CSharpGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator for CSharpGenerator {
    fn name(&self) -> &str {
        "csharp"
    }

    fn target_language(&self) -> GeneratedLanguage {
        GeneratedLanguage::CSharp
    }

    fn description(&self) -> &str {
        "Generates C# code from protobuf definitions"
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
