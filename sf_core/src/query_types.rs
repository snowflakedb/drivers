use snafu::{Location, Snafu};

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum QueryTypesError {
    #[snafu(display("Invalid precision: {precision} exceeds maximum"))]
    InvalidPrecision {
        precision: u64,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid scale: {scale} exceeds precision {precision}"))]
    InvalidScale {
        precision: u64,
        scale: u64,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid time precision: {precision} must be 0-9"))]
    InvalidTimePrecision {
        precision: u8,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Complete Snowflake type system mapped to native Arrow types
#[derive(Debug, Clone)]
pub enum RowType {
    // Numeric types (5 types)
    /// NUMBER/NUMERIC/DECIMAL/INT/INTEGER with scale=0 → Arrow Int64/Int128
    Fixed {
        name: String,
        nullable: bool,
        precision: u64,
        scale: u64,
        original_type: Option<String>,
    },
    /// NUMBER/NUMERIC/DECIMAL with scale>0 → Arrow Decimal128
    Decimal {
        name: String,
        nullable: bool,
        precision: u64,
        scale: u64,
    },
    /// REAL/FLOAT4 → Arrow Float32
    Real { name: String, nullable: bool },
    /// DOUBLE/FLOAT/FLOAT8 → Arrow Float64
    Double { name: String, nullable: bool },

    // String types (1 type)
    /// VARCHAR/CHAR/STRING/TEXT → Arrow Utf8
    Text {
        name: String,
        nullable: bool,
        length: u64,
        byte_length: u64,
    },

    // Binary types (2 types)
    /// BINARY → Arrow Binary (fixed size)
    Binary {
        name: String,
        nullable: bool,
        length: u64,
    },
    /// VARBINARY → Arrow Binary (variable size)
    VarBinary {
        name: String,
        nullable: bool,
        max_length: u64,
    },

    // Boolean type (1 type)
    /// BOOLEAN → Arrow Boolean
    Boolean { name: String, nullable: bool },

    // Date/Time types (5 types)
    /// DATE → Arrow Date32 (days since epoch)
    Date { name: String, nullable: bool },
    /// TIME → Arrow Time64 (nanoseconds since midnight)
    Time {
        name: String,
        nullable: bool,
        precision: u8, // 0-9 for fractional seconds
    },
    /// TIMESTAMP_NTZ → Arrow Timestamp (nanosecond, no timezone)
    TimestampNtz {
        name: String,
        nullable: bool,
        precision: u8,
    },
    /// TIMESTAMP_LTZ → Arrow Timestamp (nanosecond, local timezone)
    TimestampLtz {
        name: String,
        nullable: bool,
        precision: u8,
    },
    /// TIMESTAMP_TZ → Arrow Timestamp (nanosecond, with timezone)
    TimestampTz {
        name: String,
        nullable: bool,
        precision: u8,
    },

    // Semi-structured types (3 types) - NATIVE ARROW REPRESENTATION
    /// VARIANT → Arrow Union (dense) or Struct with type discriminator
    /// Supports: null, boolean, number, string, array, object
    Variant { name: String, nullable: bool },
    /// OBJECT → Arrow Map or Struct (key-value pairs)
    Object { name: String, nullable: bool },
    /// ARRAY → Arrow List (variable length array)
    Array { name: String, nullable: bool },

    // Geospatial types (2 types)
    /// GEOGRAPHY → Arrow Binary or Struct (GeoJSON/WKT)
    Geography { name: String, nullable: bool },
    /// GEOMETRY → Arrow Binary or Struct (WKT)
    Geometry { name: String, nullable: bool },
}

impl RowType {
    // Numeric type constructors

    pub fn fixed(
        name: &str,
        nullable: bool,
        precision: u64,
        scale: u64,
    ) -> Result<Self, QueryTypesError> {
        if precision > 38 {
            return InvalidPrecisionSnafu { precision }.fail();
        }
        if scale > precision {
            return InvalidScaleSnafu { precision, scale }.fail();
        }
        Ok(RowType::Fixed {
            name: name.to_string(),
            nullable,
            precision,
            scale,
            original_type: None,
        })
    }

    pub fn fixed_with_scale_zero(name: &str, nullable: bool, precision: u64) -> Self {
        RowType::Fixed {
            name: name.to_string(),
            nullable,
            precision,
            scale: 0,
            original_type: None,
        }
    }

    pub fn decimal(
        name: &str,
        nullable: bool,
        precision: u64,
        scale: u64,
    ) -> Result<Self, QueryTypesError> {
        if precision > 38 {
            return InvalidPrecisionSnafu { precision }.fail();
        }
        if scale > precision {
            return InvalidScaleSnafu { precision, scale }.fail();
        }
        Ok(RowType::Decimal {
            name: name.to_string(),
            nullable,
            precision,
            scale,
        })
    }

    pub fn real(name: &str, nullable: bool) -> Self {
        RowType::Real {
            name: name.to_string(),
            nullable,
        }
    }

    pub fn double(name: &str, nullable: bool) -> Self {
        RowType::Double {
            name: name.to_string(),
            nullable,
        }
    }

    // String type constructor

    pub fn text(name: &str, nullable: bool, length: u64, byte_length: u64) -> Self {
        RowType::Text {
            name: name.to_string(),
            nullable,
            length,
            byte_length,
        }
    }

    // Binary type constructors

    pub fn binary(name: &str, nullable: bool, length: u64) -> Self {
        RowType::Binary {
            name: name.to_string(),
            nullable,
            length,
        }
    }

    pub fn varbinary(name: &str, nullable: bool, max_length: u64) -> Self {
        RowType::VarBinary {
            name: name.to_string(),
            nullable,
            max_length,
        }
    }

    // Boolean type constructor

    pub fn boolean(name: &str, nullable: bool) -> Self {
        RowType::Boolean {
            name: name.to_string(),
            nullable,
        }
    }

    // Date/Time type constructors

    pub fn date(name: &str, nullable: bool) -> Self {
        RowType::Date {
            name: name.to_string(),
            nullable,
        }
    }

    pub fn time(name: &str, nullable: bool, precision: u8) -> Result<Self, QueryTypesError> {
        if precision > 9 {
            return InvalidTimePrecisionSnafu { precision }.fail();
        }
        Ok(RowType::Time {
            name: name.to_string(),
            nullable,
            precision,
        })
    }

    pub fn timestamp_ntz(
        name: &str,
        nullable: bool,
        precision: u8,
    ) -> Result<Self, QueryTypesError> {
        if precision > 9 {
            return InvalidTimePrecisionSnafu { precision }.fail();
        }
        Ok(RowType::TimestampNtz {
            name: name.to_string(),
            nullable,
            precision,
        })
    }

    pub fn timestamp_ltz(
        name: &str,
        nullable: bool,
        precision: u8,
    ) -> Result<Self, QueryTypesError> {
        if precision > 9 {
            return InvalidTimePrecisionSnafu { precision }.fail();
        }
        Ok(RowType::TimestampLtz {
            name: name.to_string(),
            nullable,
            precision,
        })
    }

    pub fn timestamp_tz(
        name: &str,
        nullable: bool,
        precision: u8,
    ) -> Result<Self, QueryTypesError> {
        if precision > 9 {
            return InvalidTimePrecisionSnafu { precision }.fail();
        }
        Ok(RowType::TimestampTz {
            name: name.to_string(),
            nullable,
            precision,
        })
    }

    // Semi-structured type constructors

    pub fn variant(name: &str, nullable: bool) -> Self {
        RowType::Variant {
            name: name.to_string(),
            nullable,
        }
    }

    pub fn object(name: &str, nullable: bool) -> Self {
        RowType::Object {
            name: name.to_string(),
            nullable,
        }
    }

    pub fn array(name: &str, nullable: bool) -> Self {
        RowType::Array {
            name: name.to_string(),
            nullable,
        }
    }

    // Geospatial type constructors

    pub fn geography(name: &str, nullable: bool) -> Self {
        RowType::Geography {
            name: name.to_string(),
            nullable,
        }
    }

    pub fn geometry(name: &str, nullable: bool) -> Self {
        RowType::Geometry {
            name: name.to_string(),
            nullable,
        }
    }

    // Helper method to get the type name for debugging/logging
    pub fn type_name(&self) -> &'static str {
        match self {
            RowType::Fixed { .. } => "FIXED",
            RowType::Decimal { .. } => "DECIMAL",
            RowType::Real { .. } => "REAL",
            RowType::Double { .. } => "DOUBLE",
            RowType::Text { .. } => "TEXT",
            RowType::Binary { .. } => "BINARY",
            RowType::VarBinary { .. } => "VARBINARY",
            RowType::Boolean { .. } => "BOOLEAN",
            RowType::Date { .. } => "DATE",
            RowType::Time { .. } => "TIME",
            RowType::TimestampNtz { .. } => "TIMESTAMP_NTZ",
            RowType::TimestampLtz { .. } => "TIMESTAMP_LTZ",
            RowType::TimestampTz { .. } => "TIMESTAMP_TZ",
            RowType::Variant { .. } => "VARIANT",
            RowType::Object { .. } => "OBJECT",
            RowType::Array { .. } => "ARRAY",
            RowType::Geography { .. } => "GEOGRAPHY",
            RowType::Geometry { .. } => "GEOMETRY",
        }
    }

    // Helper method to get field name
    pub fn name(&self) -> &str {
        match self {
            RowType::Fixed { name, .. }
            | RowType::Decimal { name, .. }
            | RowType::Real { name, .. }
            | RowType::Double { name, .. }
            | RowType::Text { name, .. }
            | RowType::Binary { name, .. }
            | RowType::VarBinary { name, .. }
            | RowType::Boolean { name, .. }
            | RowType::Date { name, .. }
            | RowType::Time { name, .. }
            | RowType::TimestampNtz { name, .. }
            | RowType::TimestampLtz { name, .. }
            | RowType::TimestampTz { name, .. }
            | RowType::Variant { name, .. }
            | RowType::Object { name, .. }
            | RowType::Array { name, .. }
            | RowType::Geography { name, .. }
            | RowType::Geometry { name, .. } => name,
        }
    }
}

impl RowType {
    pub fn with_original_type(mut self, original_type: Option<String>) -> Self {
        if let RowType::Fixed {
            original_type: orig,
            ..
        } = &mut self
        {
            *orig = original_type;
        }
        self
    }
}
