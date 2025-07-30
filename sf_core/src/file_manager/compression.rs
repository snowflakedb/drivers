use super::types::CompressionError;
use flate2::{Compression, GzBuilder};
use std::io::Write;

pub fn compress_data(input_data: Vec<u8>, filename: &str) -> Result<Vec<u8>, CompressionError> {
    // Python Connector adds a "_c" suffix to the filename and replaces it with spaces
    // To match that behavior, we replace the filename with spaces + 2
    let spaces_filename = " ".repeat(filename.len() + 2);

    // Use GzBuilder to create gzip with spaces filename and zeroed timestamp
    let mut encoder = GzBuilder::new()
        .mtime(0) // Set timestamp to 0 for consistent normalization
        .filename(spaces_filename) // TODO: remove this when we have more PUT/GET tests
        .write(Vec::new(), Compression::best());

    encoder.write_all(&input_data)?;
    let compressed_data = encoder.finish()?;

    Ok(compressed_data)
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{bytes_to_hex, hex_to_bytes};
    use super::*;

    #[test]
    fn test_compress_test_normal_put_csv() {
        let content = "1,2,3\n";
        let expected_content_hex = "312c322c330a";

        // Expected content after compression (hex bytes):
        let expected_compressed_hex = "1f8b08080000000002ff2020202020202020202020202020202020202020200033d431d231e602002eb41e0506000000";

        // Create a temporary directory and file with exact name "test_normal_put.csv"
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_normal_put.csv");
        std::fs::write(&file_path, content.as_bytes()).unwrap();

        let file_path = file_path.to_str().unwrap();

        // Verify content before compression
        let content_hex = bytes_to_hex(content.as_bytes());

        // Verify content hex matches expected
        assert_eq!(
            content_hex, expected_content_hex,
            "Content hex should be 312c322c330a (1,2,3\\n)"
        );

        // Read the file content as bytes
        let file_content = std::fs::read(file_path).expect("Failed to read file");

        // Extract just the filename from the path
        let filename = file_path.split('/').next_back().unwrap();

        // Compress the file using our compress_data function
        let compressed_data =
            compress_data(file_content, filename).expect("Compression should succeed");

        // Convert result to hex for comparison
        let result_hex = bytes_to_hex(&compressed_data);

        // Convert expected hex to bytes for comparison
        let expected = hex_to_bytes(expected_compressed_hex).expect("Invalid expected hex");

        // Verify the compressed output matches exactly
        assert_eq!(
            compressed_data, expected,
            "Compressed output doesn't match expected result.\nExpected: {expected_compressed_hex}\nActual:   {result_hex}"
        );
    }
}
