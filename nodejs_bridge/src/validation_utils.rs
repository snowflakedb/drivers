use crate::error::BridgeError;

pub(crate) fn is_valid_uuid(uuid: &str) -> bool {
    let bytes = uuid.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    bytes.iter().enumerate().all(|(i, &b)| match i {
        8 | 13 | 18 | 23 => b == b'-',
        _ => b.is_ascii_hexdigit(),
    })
}

pub(crate) fn require_valid_query_id(query_id: &str) -> Result<(), BridgeError> {
    if is_valid_uuid(query_id) {
        Ok(())
    } else {
        Err(BridgeError::InvalidQueryId(query_id.to_string()))
    }
}

pub(crate) fn require_valid_request_id(request_id: &str) -> Result<(), BridgeError> {
    if is_valid_uuid(request_id) {
        Ok(())
    } else {
        Err(BridgeError::InvalidRequestId(request_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_accepts_hyphenated_hex_of_either_case() {
        assert!(require_valid_query_id("01c715df-0e1a-450a-000c-a913e79f70cb").is_ok());
        assert!(require_valid_query_id("12345678-1234-4123-A123-123456789012").is_ok());
        assert!(require_valid_query_id("00000000-0000-0000-0000-000000000000").is_ok());
    }

    #[test]
    fn uuid_rejects_malformed_values() {
        for id in [
            "invalidQueryId",
            "",
            "01c715df0e1a450a000ca913e79f70cb",
            "{01c715df-0e1a-450a-000c-a913e79f70cb}",
            "01c715df-0e1a-450a-000c-a913e79f70cb ",
        ] {
            assert!(
                matches!(
                    require_valid_query_id(id),
                    Err(BridgeError::InvalidQueryId(got)) if got == id
                ),
                "{id}"
            );
            assert!(
                matches!(
                    require_valid_request_id(id),
                    Err(BridgeError::InvalidRequestId(got)) if got == id
                ),
                "{id}"
            );
        }
    }
}
