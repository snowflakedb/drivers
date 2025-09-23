use std::path::PathBuf;

use prost::Message;
use snafu::{Whatever, prelude::*};

use crate::protobuf::FileDescriptorSet;

pub(super) fn to_rust_message_name(package: &String, name: &str) -> String {
    let segments = name.split('.').collect::<Vec<&str>>();

    if segments.len() == 2 {
        return segments[1].to_string();
    }

    if segments[1] == package {
        return segments[2..].join("::");
    }

    segments[1..].join("::")
}

pub(super) fn camel_to_snake_case(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(s.len() + s.len() / 2);
    let mut chars = s.chars().peekable();

    while let Some(current_char) = chars.next() {
        // Add the current character in lowercase
        result.extend(current_char.to_lowercase());

        // Peek at the next character to decide if an underscore is needed
        if let Some(&next_char) = chars.peek() {
            // Add an underscore if transitioning from a non-uppercase
            // character to an uppercase one.
            if next_char.is_uppercase() && !current_char.is_uppercase() {
                result.push('_');
            }
        }
    }

    result
}

pub(super) fn run_protoc(input_file: PathBuf) -> Result<FileDescriptorSet, Whatever> {
    let output_dir = tempfile::tempdir().whatever_context("Failed to create temp directory")?;
    let input_filename = input_file
        .file_name()
        .with_whatever_context(|| "Failed to get input file name")?
        .to_str()
        .with_whatever_context(|| "Failed to get input file name")?;
    let output_path = output_dir
        .path()
        .join(format!("{}.compiled", input_filename));
    let output = std::process::Command::new("protoc")
        .arg(input_file.to_str().unwrap())
        .arg("-o")
        .arg(output_path.clone())
        .output()
        .whatever_context("Failed to run protoc")?;

    if !output.status.success() {
        snafu::whatever!("protoc failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let proto_bytes =
        std::fs::read(&output_path).whatever_context("Failed to read protoc output")?;

    let descriptor_set = FileDescriptorSet::decode(&*proto_bytes)
        .whatever_context("Failed to parse FileDescriptorSet")?;

    Ok(descriptor_set)
}
