use odbc_sys as sql;

use super::interval::{day_time_subtype_from_sql, year_month_subtype_from_sql};
use crate::api::CDataType;
use crate::api::types::{
    SQL_SF_ARRAY, SQL_SF_OBJECT, SQL_SF_TIMESTAMP_LTZ, SQL_SF_TIMESTAMP_NTZ, SQL_SF_TIMESTAMP_TZ,
    SQL_SF_VARIANT,
};

/// The C type that `SQL_C_DEFAULT` stands for on a parameter or column of
/// `sql_type`. `None` means the driver does not support `sql_type`.
pub(crate) fn default_c_type(sql_type: sql::SqlDataType) -> Option<CDataType> {
    let c_type = match sql_type {
        sql::SqlDataType::CHAR
        | sql::SqlDataType::VARCHAR
        | sql::SqlDataType::EXT_LONG_VARCHAR
        | sql::SqlDataType::DECIMAL
        | sql::SqlDataType::NUMERIC => CDataType::Char,
        // Only catalog text columns report a wide SQL type, so query text columns still default to SQL_C_CHAR.
        sql::SqlDataType::EXT_W_CHAR
        | sql::SqlDataType::EXT_W_VARCHAR
        | sql::SqlDataType::EXT_W_LONG_VARCHAR => CDataType::WChar,
        sql::SqlDataType::SMALLINT => CDataType::SShort,
        sql::SqlDataType::INTEGER => CDataType::SLong,
        sql::SqlDataType::EXT_TINY_INT => CDataType::STinyInt,
        sql::SqlDataType::EXT_BIG_INT => CDataType::SBigInt,
        // Appendix D gives SQL_REAL a 4-byte default, unlike SQL_FLOAT and SQL_DOUBLE.
        sql::SqlDataType::REAL => CDataType::Float,
        sql::SqlDataType::FLOAT | sql::SqlDataType::DOUBLE => CDataType::Double,
        sql::SqlDataType::EXT_BIT => CDataType::Bit,
        sql::SqlDataType::EXT_BINARY
        | sql::SqlDataType::EXT_VAR_BINARY
        | sql::SqlDataType::EXT_LONG_VAR_BINARY => CDataType::Binary,
        sql::SqlDataType::DATE | sql::SqlDataType::DATETIME => CDataType::Date,
        sql::SqlDataType::TIME | sql::SqlDataType::EXT_TIME_OR_INTERVAL => CDataType::Time,
        sql::SqlDataType::TIMESTAMP
        | sql::SqlDataType::EXT_TIMESTAMP
        | SQL_SF_TIMESTAMP_NTZ
        | SQL_SF_TIMESTAMP_LTZ
        | SQL_SF_TIMESTAMP_TZ => CDataType::TimeStamp,
        // Only fetch reaches these; SQLBindParameter rejects them with HYC00.
        SQL_SF_ARRAY | SQL_SF_OBJECT | SQL_SF_VARIANT => CDataType::Char,
        other
            if year_month_subtype_from_sql(other.0).is_some()
                || day_time_subtype_from_sql(other.0).is_some() =>
        {
            CDataType::Char
        }
        _ => return None,
    };
    Some(c_type)
}

/// Replaces `SQL_C_DEFAULT` with [`default_c_type`] of `sql_type`; any other
/// C type is returned unchanged.
pub(crate) fn resolve_default_c_type(c_type: CDataType, sql_type: sql::SqlDataType) -> CDataType {
    match c_type {
        // Without a default, SQL_C_DEFAULT stays unresolved and conversion rejects it.
        CDataType::Default => default_c_type(sql_type).unwrap_or(c_type),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::{default_c_type, resolve_default_c_type};
    use crate::api::CDataType;
    use crate::api::types::{
        SQL_SF_ARRAY, SQL_SF_OBJECT, SQL_SF_TIMESTAMP_LTZ, SQL_SF_TIMESTAMP_NTZ,
        SQL_SF_TIMESTAMP_TZ, SQL_SF_VARIANT,
    };
    use odbc_sys as sql;

    #[test]
    fn default_c_type_maps_every_supported_sql_type() {
        let cases = [
            (sql::SqlDataType::CHAR, CDataType::Char),
            (sql::SqlDataType::VARCHAR, CDataType::Char),
            (sql::SqlDataType::EXT_LONG_VARCHAR, CDataType::Char),
            (sql::SqlDataType::DECIMAL, CDataType::Char),
            (sql::SqlDataType::NUMERIC, CDataType::Char),
            (sql::SqlDataType::EXT_W_CHAR, CDataType::WChar),
            (sql::SqlDataType::EXT_W_VARCHAR, CDataType::WChar),
            (sql::SqlDataType::EXT_W_LONG_VARCHAR, CDataType::WChar),
            (sql::SqlDataType::SMALLINT, CDataType::SShort),
            (sql::SqlDataType::INTEGER, CDataType::SLong),
            (sql::SqlDataType::EXT_TINY_INT, CDataType::STinyInt),
            (sql::SqlDataType::EXT_BIG_INT, CDataType::SBigInt),
            (sql::SqlDataType::REAL, CDataType::Float),
            (sql::SqlDataType::FLOAT, CDataType::Double),
            (sql::SqlDataType::DOUBLE, CDataType::Double),
            (sql::SqlDataType::EXT_BIT, CDataType::Bit),
            (sql::SqlDataType::EXT_BINARY, CDataType::Binary),
            (sql::SqlDataType::EXT_VAR_BINARY, CDataType::Binary),
            (sql::SqlDataType::EXT_LONG_VAR_BINARY, CDataType::Binary),
            (sql::SqlDataType::DATE, CDataType::Date),
            (sql::SqlDataType::DATETIME, CDataType::Date),
            (sql::SqlDataType::TIME, CDataType::Time),
            (sql::SqlDataType::EXT_TIME_OR_INTERVAL, CDataType::Time),
            (sql::SqlDataType::TIMESTAMP, CDataType::TimeStamp),
            (sql::SqlDataType::EXT_TIMESTAMP, CDataType::TimeStamp),
            (SQL_SF_TIMESTAMP_NTZ, CDataType::TimeStamp),
            (SQL_SF_TIMESTAMP_LTZ, CDataType::TimeStamp),
            (SQL_SF_TIMESTAMP_TZ, CDataType::TimeStamp),
            (SQL_SF_ARRAY, CDataType::Char),
            (SQL_SF_OBJECT, CDataType::Char),
            (SQL_SF_VARIANT, CDataType::Char),
        ];
        for (sql_type, expected) in cases {
            assert_eq!(default_c_type(sql_type), Some(expected), "{sql_type:?}");
        }
        for code in 101..=113 {
            assert_eq!(
                default_c_type(sql::SqlDataType(code)),
                Some(CDataType::Char),
                "interval code {code}"
            );
        }
    }

    #[test]
    fn default_c_type_has_no_default_for_unsupported_sql_types() {
        for sql_type in [
            sql::SqlDataType::EXT_GUID,
            sql::SqlDataType(94),
            sql::SqlDataType(95),
            sql::SqlDataType::UNKNOWN_TYPE,
        ] {
            assert_eq!(default_c_type(sql_type), None, "{sql_type:?}");
        }
    }

    #[test]
    fn resolve_default_c_type_keeps_explicit_c_types() {
        assert_eq!(
            resolve_default_c_type(CDataType::WChar, sql::SqlDataType::INTEGER),
            CDataType::WChar
        );
    }

    #[test]
    fn resolve_default_c_type_keeps_sql_c_default_without_a_default() {
        assert_eq!(
            resolve_default_c_type(CDataType::Default, sql::SqlDataType::EXT_GUID),
            CDataType::Default
        );
    }

    #[test]
    fn resolve_default_c_type_resolves_sql_c_default() {
        assert_eq!(
            resolve_default_c_type(CDataType::Default, sql::SqlDataType::REAL),
            CDataType::Float
        );
    }
}
