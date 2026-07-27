//! Shared helpers for the S3/Azure/GCS multipart integration tests
//! (`s3_multipart.rs`, `azure_multipart.rs`, `gcs_multipart.rs`).

/// Deterministic, position-dependent payload (a tiny LCG) so that a mis-ordered
/// chunk/block/part or range on reassembly cannot still compare equal to the
/// original.
pub(crate) fn make_payload(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut state: u32 = 0x9e37_79b9;
    for _ in 0..len {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        out.push((state >> 24) as u8);
    }
    out
}

/// Parse an inclusive `Range: bytes=START-END` header against `total`.
pub(crate) fn parse_range(value: &str, total: usize) -> (usize, usize) {
    let spec = value.trim().trim_start_matches("bytes=");
    let mut it = spec.split('-');
    let start: usize = it.next().unwrap().trim().parse().unwrap();
    let end: usize = it
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap())
        .unwrap_or(total - 1);
    (start, end.min(total - 1))
}
