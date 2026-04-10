use crate::rest::snowflake::query_response::Data;

const STATEMENT_TYPE_MULTI: i64 = 0xA000;

/// Returns `true` if the response represents a multi-statement execution.
pub fn is_multistatement(data: &Data) -> bool {
    data.statement_type_id == Some(STATEMENT_TYPE_MULTI)
}

/// Parse comma-separated `resultIds` from a multi-statement response into individual query IDs.
pub fn child_query_ids(data: &Data) -> Vec<String> {
    data.result_ids
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|ids| ids.split(',').map(|id| id.trim().to_string()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comma_separated_ids() {
        let data = data_with_result_ids(Some("id1,id2,id3".to_string()));
        assert_eq!(child_query_ids(&data), vec!["id1", "id2", "id3"]);
    }

    #[test]
    fn handles_whitespace_in_ids() {
        let data = data_with_result_ids(Some("id1, id2 , id3".to_string()));
        assert_eq!(child_query_ids(&data), vec!["id1", "id2", "id3"]);
    }

    #[test]
    fn returns_empty_for_none() {
        let data = data_with_result_ids(None);
        assert!(child_query_ids(&data).is_empty());
    }

    #[test]
    fn returns_empty_for_empty_string() {
        let data = data_with_result_ids(Some(String::new()));
        assert!(child_query_ids(&data).is_empty());
    }

    #[test]
    fn detects_multistatement_type() {
        let json = serde_json::json!({ "statementTypeId": 0xA000 });
        let data: Data = serde_json::from_value(json).unwrap();
        assert!(is_multistatement(&data));
    }

    #[test]
    fn not_multistatement_for_regular_query() {
        let json = serde_json::json!({ "statementTypeId": 4096 });
        let data: Data = serde_json::from_value(json).unwrap();
        assert!(!is_multistatement(&data));
    }

    #[test]
    fn not_multistatement_when_missing() {
        let json = serde_json::json!({});
        let data: Data = serde_json::from_value(json).unwrap();
        assert!(!is_multistatement(&data));
    }

    fn data_with_result_ids(result_ids: Option<String>) -> Data {
        let mut json = serde_json::json!({});
        if let Some(ids) = result_ids {
            json["resultIds"] = serde_json::Value::String(ids);
        }
        serde_json::from_value(json).unwrap()
    }
}
