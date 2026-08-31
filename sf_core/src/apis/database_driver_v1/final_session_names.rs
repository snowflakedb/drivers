use crate::rest::snowflake::query_response;

/// Session-level identifiers reported by the server after each query.
/// Used to keep the client's view of current database/schema/warehouse/role
/// in sync with server-side changes (e.g. `USE DATABASE`, HTAP optimization).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FinalSessionNames {
    pub database: Option<String>,
    pub schema: Option<String>,
    pub warehouse: Option<String>,
    pub role: Option<String>,
}

impl From<&query_response::Data> for FinalSessionNames {
    fn from(data: &query_response::Data) -> Self {
        Self {
            database: data.final_database_name.clone(),
            schema: data.final_schema_name.clone(),
            warehouse: data.final_warehouse_name.clone(),
            role: data.final_role_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(
        db: Option<&str>,
        schema: Option<&str>,
        wh: Option<&str>,
        role: Option<&str>,
    ) -> query_response::Data {
        let mut data = query_response::Data::default();
        data.final_database_name = db.map(String::from);
        data.final_schema_name = schema.map(String::from);
        data.final_warehouse_name = wh.map(String::from);
        data.final_role_name = role.map(String::from);
        data
    }

    #[test]
    fn from_data_with_all_fields() {
        let data = make_data(
            Some("MY_DB"),
            Some("PUBLIC"),
            Some("COMPUTE_WH"),
            Some("SYSADMIN"),
        );

        let names = FinalSessionNames::from(&data);

        assert_eq!(names.database.as_deref(), Some("MY_DB"));
        assert_eq!(names.schema.as_deref(), Some("PUBLIC"));
        assert_eq!(names.warehouse.as_deref(), Some("COMPUTE_WH"));
        assert_eq!(names.role.as_deref(), Some("SYSADMIN"));
    }

    #[test]
    fn from_data_with_no_fields() {
        let data = make_data(None, None, None, None);

        let names = FinalSessionNames::from(&data);

        assert_eq!(names.database, None);
        assert_eq!(names.schema, None);
        assert_eq!(names.warehouse, None);
        assert_eq!(names.role, None);
    }

    #[test]
    fn from_data_with_partial_fields() {
        let data = make_data(Some("DB"), None, Some("WH"), None);

        let names = FinalSessionNames::from(&data);

        assert_eq!(names.database.as_deref(), Some("DB"));
        assert_eq!(names.schema, None);
        assert_eq!(names.warehouse.as_deref(), Some("WH"));
        assert_eq!(names.role, None);
    }

    #[test]
    fn default_is_all_none() {
        let names = FinalSessionNames::default();

        assert_eq!(
            names,
            FinalSessionNames {
                database: None,
                schema: None,
                warehouse: None,
                role: None,
            }
        );
    }
}
