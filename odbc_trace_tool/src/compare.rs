use crate::model::OdbcCall;

/// Cost of inserting or deleting a call in the edit distance.
const INDEL_COST: f64 = 1.0;

/// Filter out diagnostic noise that varies across driver managers.
pub fn filter_for_comparison(calls: &[OdbcCall]) -> Vec<&OdbcCall> {
    calls
        .iter()
        .filter(|c| !matches!(c, OdbcCall::GetDiagRec(_) | OdbcCall::GetFunctions(_)))
        .collect()
}

/// Substitution cost between two calls.
///
/// - Different function name: 1.0
/// - Same function: sum of per-field penalties (capped at 1.0)
pub fn call_distance(a: &OdbcCall, b: &OdbcCall) -> f64 {
    if std::mem::discriminant(a) != std::mem::discriminant(b) {
        return 1.0;
    }

    let penalty = match (a, b) {
        (OdbcCall::AllocHandle(a), OdbcCall::AllocHandle(b)) => {
            diff_rc(a.return_code, b.return_code) + diff_opt(&a.handle_type, &b.handle_type, 1.0)
        }
        (OdbcCall::FreeHandle(a), OdbcCall::FreeHandle(b)) => {
            diff_rc(a.return_code, b.return_code) + diff_opt(&a.handle_type, &b.handle_type, 1.0)
        }
        (OdbcCall::SetEnvAttr(a), OdbcCall::SetEnvAttr(b)) => {
            diff_rc(a.return_code, b.return_code)
                + diff_opt(&a.attribute, &b.attribute, 0.3)
                + diff_opt(&a.value, &b.value, 0.1)
        }
        (OdbcCall::SetConnectAttr(a), OdbcCall::SetConnectAttr(b)) => {
            diff_rc(a.return_code, b.return_code)
                + diff_opt(&a.attribute, &b.attribute, 0.3)
                + diff_opt(&a.value, &b.value, 0.1)
        }
        (OdbcCall::Prepare(a), OdbcCall::Prepare(b)) => {
            diff_rc(a.return_code, b.return_code) + diff_opt(&a.sql, &b.sql, 0.4)
        }
        (OdbcCall::ExecDirect(a), OdbcCall::ExecDirect(b)) => {
            diff_rc(a.return_code, b.return_code) + diff_opt(&a.sql, &b.sql, 0.4)
        }
        (OdbcCall::DescribeCol(a), OdbcCall::DescribeCol(b)) => {
            diff_rc(a.return_code, b.return_code)
                + diff_opt(&a.column_number, &b.column_number, 0.2)
                + diff_opt(&a.data_type, &b.data_type, 0.1)
        }
        (OdbcCall::GetData(a), OdbcCall::GetData(b)) => {
            diff_rc(a.return_code, b.return_code)
                + diff_opt(&a.column_number, &b.column_number, 0.2)
                + diff_opt(&a.target_type_name, &b.target_type_name, 1.0)
        }
        (OdbcCall::FetchScroll(a), OdbcCall::FetchScroll(b)) => {
            diff_rc(a.return_code, b.return_code)
                + diff_opt(&a.orientation_name, &b.orientation_name, 0.2)
        }
        (OdbcCall::GetInfo(a), OdbcCall::GetInfo(b)) => {
            diff_rc(a.return_code, b.return_code) + diff_opt(&a.info_type, &b.info_type, 1.0)
        }
        (OdbcCall::NumResultCols(a), OdbcCall::NumResultCols(b)) => {
            diff_rc(a.return_code, b.return_code) + diff_opt(&a.count, &b.count, 0.1)
        }
        (OdbcCall::RowCount(a), OdbcCall::RowCount(b)) => {
            diff_rc(a.return_code, b.return_code) + diff_opt(&a.count, &b.count, 0.1)
        }
        // Calls with no distinguishing args beyond return_code.
        _ => diff_rc(a.return_code(), b.return_code()),
    };

    penalty.min(1.0)
}

fn diff_rc(a: crate::model::ReturnCode, b: crate::model::ReturnCode) -> f64 {
    if a == b {
        0.0
    } else {
        1.0
    }
}

fn diff_opt<T: PartialEq>(a: &Option<T>, b: &Option<T>, weight: f64) -> f64 {
    match (a, b) {
        (Some(a), Some(b)) if a == b => 0.0,
        (None, None) => 0.0,
        _ => weight,
    }
}

/// Weighted Levenshtein edit distance on two call sequences, normalized to [0.0, 1.0].
pub fn compare_traces(a: &[&OdbcCall], b: &[&OdbcCall]) -> f64 {
    let n = a.len();
    let m = b.len();
    if n == 0 && m == 0 {
        return 0.0;
    }

    // Two-row DP. prev = row i-1, curr = row i.
    let mut prev: Vec<f64> = (0..=m).map(|j| j as f64 * INDEL_COST).collect();
    let mut curr = vec![0.0; m + 1];

    for i in 1..=n {
        curr[0] = i as f64 * INDEL_COST;
        for j in 1..=m {
            let sub = prev[j - 1] + call_distance(a[i - 1], b[j - 1]);
            let del = prev[j] + INDEL_COST;
            let ins = curr[j - 1] + INDEL_COST;
            curr[j] = sub.min(del).min(ins);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    let raw = prev[m];
    let max_len = n.max(m) as f64;
    raw / max_len
}

pub struct CompareResult {
    pub name_a: String,
    pub name_b: String,
    pub distance: f64,
    pub len_b: usize,
}

pub fn print_report(reference_label: &str, results: &mut [CompareResult]) {
    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
    let multi_ref = results.windows(2).any(|w| w[0].name_a != w[1].name_a);
    println!("Reference: {reference_label}");
    for r in results.iter() {
        let identical = if r.distance == 0.0 {
            " (identical)"
        } else {
            ""
        };
        let best_ref = if multi_ref {
            format!(" [closest: {}]", r.name_a)
        } else {
            String::new()
        };
        println!(
            "  vs {} ({} calls): distance {:.3}{identical}{best_ref}",
            r.name_b, r.len_b, r.distance
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn prep(sql: &str) -> OdbcCall {
        OdbcCall::Prepare(Prepare {
            return_code: ReturnCode::Success,
            handle: None,
            sql: Some(sql.to_string()),
            sql_truncated: false,
        })
    }

    fn exec() -> OdbcCall {
        OdbcCall::Execute(Execute {
            return_code: ReturnCode::Success,
            handle: None,
        })
    }

    fn fetch_ok() -> OdbcCall {
        OdbcCall::Fetch(Fetch {
            return_code: ReturnCode::Success,
            handle: None,
        })
    }

    fn fetch_no_data() -> OdbcCall {
        OdbcCall::Fetch(Fetch {
            return_code: ReturnCode::NoData,
            handle: None,
        })
    }

    #[test]
    fn test_identical_traces() {
        let a = vec![prep("SELECT 1"), exec(), fetch_ok()];
        let b = vec![prep("SELECT 1"), exec(), fetch_ok()];
        let fa: Vec<_> = a.iter().collect();
        let fb: Vec<_> = b.iter().collect();
        assert_eq!(compare_traces(&fa, &fb), 0.0);
    }

    #[test]
    fn test_completely_different() {
        let a = [prep("SELECT 1")];
        let b = [exec()];
        let fa: Vec<_> = a.iter().collect();
        let fb: Vec<_> = b.iter().collect();
        assert_eq!(compare_traces(&fa, &fb), 1.0);
    }

    #[test]
    fn test_same_function_different_sql() {
        let d = call_distance(&prep("SELECT 1"), &prep("SELECT 2"));
        assert!((d - 0.4).abs() < 1e-9, "expected 0.4, got {d}");
    }

    #[test]
    fn test_same_function_different_return_code() {
        let d = call_distance(&fetch_ok(), &fetch_no_data());
        assert!((d - 1.0).abs() < 1e-9, "expected 1.0, got {d}");
    }

    #[test]
    fn test_same_function_same_args() {
        let d = call_distance(&exec(), &exec());
        assert_eq!(d, 0.0);
    }

    #[test]
    fn test_different_function() {
        let d = call_distance(&prep("SELECT 1"), &exec());
        assert_eq!(d, 1.0);
    }

    #[test]
    fn test_insertion_deletion() {
        let a = vec![prep("SELECT 1"), exec(), fetch_ok()];
        let b = vec![prep("SELECT 1"), exec()];
        let fa: Vec<_> = a.iter().collect();
        let fb: Vec<_> = b.iter().collect();
        let d = compare_traces(&fa, &fb);
        assert!((d - 1.0 / 3.0).abs() < 1e-9, "expected ~0.333, got {d}");
    }

    #[test]
    fn test_empty_vs_nonempty() {
        let a: Vec<OdbcCall> = vec![];
        let b = [exec()];
        let fa: Vec<_> = a.iter().collect();
        let fb: Vec<_> = b.iter().collect();
        assert_eq!(compare_traces(&fa, &fb), 1.0);
    }

    #[test]
    fn test_both_empty() {
        let a: Vec<OdbcCall> = vec![];
        let b: Vec<OdbcCall> = vec![];
        let fa: Vec<_> = a.iter().collect();
        let fb: Vec<_> = b.iter().collect();
        assert_eq!(compare_traces(&fa, &fb), 0.0);
    }

    #[test]
    fn test_penalty_caps_at_one() {
        let a = OdbcCall::SetEnvAttr(SetEnvAttr {
            return_code: ReturnCode::Success,
            handle: None,
            attribute: Some("SQL_ATTR_ODBC_VERSION".to_string()),
            value: Some(3),
            str_len: None,
        });
        let b = OdbcCall::SetEnvAttr(SetEnvAttr {
            return_code: ReturnCode::Error,
            handle: None,
            attribute: Some("SQL_ATTR_AUTOCOMMIT".to_string()),
            value: Some(1),
            str_len: None,
        });
        let d = call_distance(&a, &b);
        assert!((d - 1.0).abs() < 1e-9, "should cap at 1.0, got {d}");
    }
}
