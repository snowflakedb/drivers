use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::model::OdbcCall;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMapEntry {
    pub index: usize,
    pub original: String,
    pub mapped: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMap {
    pub queries: Vec<QueryMapEntry>,
}

impl QueryMap {
    pub fn get(&self, index: usize) -> Option<&str> {
        self.queries
            .iter()
            .find(|e| e.index == index)
            .map(|e| e.mapped.as_str())
    }
}

static DTM_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^-- DTM_\w+:.*\n?").unwrap());

pub fn strip_dtm_comments(sql: &str) -> String {
    let result = DTM_COMMENT_RE.replace_all(sql, "");
    let trimmed = result.trim_start_matches('\n');
    trimmed.trim_end().to_string()
}

pub fn generate_query_map(calls: &[OdbcCall]) -> QueryMap {
    let mut entries = Vec::new();
    let mut index = 0;

    for call in calls {
        let original = match call {
            OdbcCall::Prepare(c) => c.sql.clone().unwrap_or_default(),
            OdbcCall::ExecDirect(c) => c.sql.clone().unwrap_or_default(),
            _ => continue,
        };

        let mapped = strip_dtm_comments(&original);

        entries.push(QueryMapEntry {
            index,
            original,
            mapped,
        });
        index += 1;
    }

    QueryMap { queries: entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_dtm_comments_all_three_tags() {
        let sql = "-- DTM_BRID: 652649af-6392-434e-9227-7d5ff972d577\n\
                    -- DTM_QRY:\n\
                    -- DTM_BSID: 652649af-6392-434e-9227-7d5ff972d577.0001 \n\
                    SELECT\n   ...";
        assert_eq!(strip_dtm_comments(sql), "SELECT\n   ...");
    }

    #[test]
    fn test_strip_dtm_comments_brid_and_qry_only() {
        let sql = "-- DTM_BRID: abc-123\n\
                    -- DTM_QRY:\n\
                    SHOW SCHEMAS LIKE 'PUBLIC'";
        assert_eq!(strip_dtm_comments(sql), "SHOW SCHEMAS LIKE 'PUBLIC'");
    }

    #[test]
    fn test_strip_dtm_comments_no_dtm() {
        let sql = "SELECT 1";
        assert_eq!(strip_dtm_comments(sql), "SELECT 1");
    }

    #[test]
    fn test_strip_dtm_comments_preserves_non_dtm_comments() {
        let sql = "-- DTM_BRID: abc\n-- regular comment\nSELECT 1";
        assert_eq!(strip_dtm_comments(sql), "-- regular comment\nSELECT 1");
    }

    #[test]
    fn test_strip_dtm_comments_empty() {
        assert_eq!(strip_dtm_comments(""), "");
    }

    #[test]
    fn test_query_map_get() {
        let qm = QueryMap {
            queries: vec![
                QueryMapEntry {
                    index: 0,
                    original: "orig0".to_string(),
                    mapped: "mapped0".to_string(),
                },
                QueryMapEntry {
                    index: 1,
                    original: "orig1".to_string(),
                    mapped: "mapped1".to_string(),
                },
            ],
        };
        assert_eq!(qm.get(0), Some("mapped0"));
        assert_eq!(qm.get(1), Some("mapped1"));
        assert_eq!(qm.get(2), None);
    }

    #[test]
    fn test_query_map_yaml_roundtrip() {
        let qm = QueryMap {
            queries: vec![
                QueryMapEntry {
                    index: 0,
                    original: "-- DTM_BRID: abc\nSELECT 1".to_string(),
                    mapped: "SELECT 1".to_string(),
                },
                QueryMapEntry {
                    index: 1,
                    original: "SHOW SCHEMAS LIKE 'PUBLIC'".to_string(),
                    mapped: "SHOW SCHEMAS LIKE 'PUBLIC'".to_string(),
                },
            ],
        };

        let yaml = serde_yaml::to_string(&qm).expect("serialize");
        let loaded: QueryMap = serde_yaml::from_str(&yaml).expect("deserialize");

        assert_eq!(loaded.queries.len(), 2);
        assert_eq!(loaded.get(0), Some("SELECT 1"));
        assert_eq!(loaded.get(1), Some("SHOW SCHEMAS LIKE 'PUBLIC'"));
    }

    #[test]
    fn test_generate_query_map_from_trace() {
        let trace = crate::parser::unixodbc::parse_str(SAMPLE_TRACE).expect("parse");
        let calls: Vec<_> = trace.calls.iter().map(|tc| tc.call.clone()).collect();
        let qm = generate_query_map(&calls);

        assert_eq!(qm.queries.len(), 1);
        assert_eq!(qm.queries[0].index, 0);
        assert_eq!(qm.queries[0].original, "SELECT 1");
        assert_eq!(qm.queries[0].mapped, "SELECT 1");
    }

    const SAMPLE_TRACE: &str = "\
[ODBC][100][1774615098.100000][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0xaaa
[ODBC][100][1774615098.200000][SQLAllocHandle.c][395]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0xaaa
[ODBC][100][1774615098.200001][SQLAllocHandle.c][531]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xbbb
[ODBC][100][1774615098.300000][SQLAllocHandle.c][578]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xbbb
[ODBC][100][1774615098.300001][SQLAllocHandle.c][1123]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xccc
[ODBC][100][1774615098.400000][SQLExecDirect.c][240]
\t\tEntry:
\t\t\tStatement = 0xccc
\t\t\tSQL = [SELECT 1][length = 8 (SQL_NTS)]
[ODBC][100][1774615098.500000][SQLExecDirect.c][521]
\t\tExit:[SQL_SUCCESS]
";
}
