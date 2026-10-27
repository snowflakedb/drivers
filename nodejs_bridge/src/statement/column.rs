use napi_derive::napi;
use sf_core::apis::database_driver_v1::ColumnMetadata;
use std::collections::{HashMap, HashSet};

#[napi]
pub struct Column {
    name: String,
    index: u32,
    nullable: bool,
    scale: Option<i64>,
    precision: Option<i64>,
    type_name: String,
}

impl Column {
    pub(super) fn from_metadata(index: u32, meta: &ColumnMetadata, name: String) -> Self {
        Self {
            name,
            index,
            nullable: meta.nullable,
            scale: meta.scale,
            precision: meta.precision,
            type_name: meta.r#type.to_lowercase(),
        }
    }
}

#[napi]
impl Column {
    #[napi]
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    #[napi]
    pub fn get_index(&self) -> u32 {
        self.index
    }

    #[napi]
    pub fn get_id(&self) -> u32 {
        self.index
    }

    #[napi]
    pub fn is_nullable(&self) -> bool {
        self.nullable
    }

    #[napi]
    pub fn get_scale(&self) -> Option<i64> {
        self.scale
    }

    #[napi]
    pub fn get_precision(&self) -> Option<i64> {
        self.precision
    }

    #[napi]
    pub fn get_type(&self) -> String {
        self.type_name.clone()
    }

    #[napi]
    pub fn is_string(&self) -> bool {
        self.type_name == "text"
    }

    #[napi]
    pub fn is_binary(&self) -> bool {
        self.type_name == "binary"
    }

    #[napi]
    pub fn is_number(&self) -> bool {
        self.type_name == "fixed" || self.type_name == "real"
    }

    #[napi]
    pub fn is_decfloat(&self) -> bool {
        self.type_name == "decfloat"
    }

    #[napi]
    pub fn is_interval_year_month(&self) -> bool {
        self.type_name == "interval_year_month"
    }

    #[napi]
    pub fn is_interval_day_time(&self) -> bool {
        self.type_name == "interval_day_time"
    }

    #[napi]
    pub fn is_boolean(&self) -> bool {
        self.type_name == "boolean"
    }

    #[napi]
    pub fn is_date(&self) -> bool {
        self.type_name == "date"
    }

    #[napi]
    pub fn is_time(&self) -> bool {
        self.type_name == "time"
    }

    #[napi]
    pub fn is_timestamp(&self) -> bool {
        self.is_timestamp_ltz() || self.is_timestamp_ntz() || self.is_timestamp_tz()
    }

    #[napi]
    pub fn is_timestamp_ltz(&self) -> bool {
        self.type_name == "timestamp_ltz"
    }

    #[napi]
    pub fn is_timestamp_ntz(&self) -> bool {
        self.type_name == "timestamp_ntz"
    }

    #[napi]
    pub fn is_timestamp_tz(&self) -> bool {
        self.type_name == "timestamp_tz"
    }

    #[napi]
    pub fn is_variant(&self) -> bool {
        matches!(
            self.type_name.as_str(),
            "variant" | "object" | "array" | "map"
        )
    }

    // We currently only support semi-structured types. The old driver
    // distinguished structured OBJECT/ARRAY/MAP from untyped semi-structured
    // columns using the server's `fields` metadata.
    //
    // Structured-type support is a separate initiative
    // tracked in https://snowflakecomputing.atlassian.net/browse/SNOW-3445814.
    #[napi]
    pub fn is_object(&self) -> bool {
        self.type_name == "object"
    }

    #[napi]
    pub fn is_array(&self) -> bool {
        self.type_name == "array"
    }

    // Semi-structured MAP columns (no structured `fields` metadata) come back as
    // `object` from the server, so the JS value is a plain object rather than a
    // JS `Map`. Only structured `::MAP(K, V)` columns (which we don't yet surface
    // field metadata for — see the TODO above) would report as a true map.
    #[napi]
    pub fn is_map(&self) -> bool {
        self.type_name == "map"
    }

    #[napi]
    pub fn is_vector(&self) -> bool {
        self.type_name == "vector"
    }
}

pub(super) fn column_names(columns: &[ColumnMetadata], rename_duplicates: bool) -> Vec<String> {
    let names: Vec<&str> = columns.iter().map(|meta| meta.name.as_str()).collect();
    if rename_duplicates {
        rename_duplicate_column_names(&names)
    } else {
        names.into_iter().map(str::to_owned).collect()
    }
}

fn rename_duplicate_column_names(names: &[&str]) -> Vec<String> {
    let mut taken: HashSet<String> = names.iter().map(|&name| name.to_owned()).collect();
    let mut occurrences: HashMap<&str, usize> = HashMap::new();
    names
        .iter()
        .map(|&name| {
            if name.is_empty() {
                return String::new();
            }
            let Some(times) = occurrences.get_mut(name) else {
                occurrences.insert(name, 1);
                return name.to_owned();
            };
            let candidate = loop {
                *times += 1;
                let candidate = format!("{name}_{times}");
                if !taken.contains(&candidate) {
                    break candidate;
                }
            };
            taken.insert(candidate.clone());
            candidate
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_names_without_duplicates() {
        assert_eq!(
            rename_duplicate_column_names(&["COL1", "COL2"]),
            ["COL1", "COL2"]
        );
    }

    #[test]
    fn should_rename_a_single_duplicated_column() {
        assert_eq!(
            rename_duplicate_column_names(&["COL1", "COL1"]),
            ["COL1", "COL1_2"]
        );
    }

    #[test]
    fn should_return_no_names_for_no_columns() {
        assert!(rename_duplicate_column_names(&[]).is_empty());
    }

    #[test]
    fn should_skip_a_suffix_that_already_exists() {
        assert_eq!(
            rename_duplicate_column_names(&["COL1", "COL1", "COL1_2"]),
            ["COL1", "COL1_3", "COL1_2"]
        );
    }

    #[test]
    fn should_rename_multiple_duplicated_columns() {
        assert_eq!(
            rename_duplicate_column_names(&[
                "COL1", "COL1", "COL2", "COL2", "COL2", "COL3", "COL3"
            ]),
            [
                "COL1", "COL1_2", "COL2", "COL2_2", "COL2_3", "COL3", "COL3_2"
            ]
        );
    }

    #[test]
    fn should_rename_duplicates_of_names_that_already_carry_a_suffix() {
        assert_eq!(
            rename_duplicate_column_names(&["COL1", "COL1_2", "COL1_2", "COL1"]),
            ["COL1", "COL1_2", "COL1_2_2", "COL1_3"]
        );
    }

    #[test]
    fn should_never_rename_empty_names() {
        assert_eq!(rename_duplicate_column_names(&["", ""]), ["", ""]);
    }

    #[test]
    fn should_chain_renames_across_repeated_and_suffixed_names() {
        assert_eq!(
            rename_duplicate_column_names(&[
                "KEY", "FOO", "KEY_1", "FOO", "KEY_3", "FOO_3", "KEY", "FOO", "KEY", "FOO"
            ]),
            [
                "KEY", "FOO", "KEY_1", "FOO_2", "KEY_3", "FOO_3", "KEY_2", "FOO_4", "KEY_4",
                "FOO_5"
            ]
        );
    }
}
