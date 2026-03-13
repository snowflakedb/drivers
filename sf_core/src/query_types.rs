pub enum RowType {
    Fixed {
        name: String,
        nullable: bool,
        precision: u64,
        scale: u64,
    },
    Text {
        name: String,
        nullable: bool,
        length: u64,
        byte_length: u64,
    },
    Boolean {
        name: String,
        nullable: bool,
    },
    Real {
        name: String,
        nullable: bool,
    },
    Date {
        name: String,
        nullable: bool,
    },
    TimestampNtz {
        name: String,
        nullable: bool,
        scale: u64,
    },
    TimestampLtz {
        name: String,
        nullable: bool,
        scale: u64,
    },
    TimestampTz {
        name: String,
        nullable: bool,
        scale: u64,
    },
}

impl RowType {
    pub fn fixed(name: &str, nullable: bool, precision: u64, scale: u64) -> Self {
        RowType::Fixed {
            name: name.to_string(),
            nullable,
            precision,
            scale,
        }
    }

    pub fn fixed_with_scale_zero(name: &str, nullable: bool, precision: u64) -> Self {
        RowType::Fixed {
            name: name.to_string(),
            nullable,
            precision,
            scale: 0,
        }
    }

    pub fn text(name: &str, nullable: bool, length: u64, byte_length: u64) -> Self {
        RowType::Text {
            name: name.to_string(),
            nullable,
            length,
            byte_length,
        }
    }

    pub fn boolean(name: &str, nullable: bool) -> Self {
        RowType::Boolean {
            name: name.to_string(),
            nullable,
        }
    }

    pub fn real(name: &str, nullable: bool) -> Self {
        RowType::Real {
            name: name.to_string(),
            nullable,
        }
    }

    pub fn date(name: &str, nullable: bool) -> Self {
        RowType::Date {
            name: name.to_string(),
            nullable,
        }
    }

    pub fn timestamp_ntz(name: &str, nullable: bool, scale: u64) -> Self {
        RowType::TimestampNtz {
            name: name.to_string(),
            nullable,
            scale,
        }
    }

    pub fn timestamp_ltz(name: &str, nullable: bool, scale: u64) -> Self {
        RowType::TimestampLtz {
            name: name.to_string(),
            nullable,
            scale,
        }
    }

    pub fn timestamp_tz(name: &str, nullable: bool, scale: u64) -> Self {
        RowType::TimestampTz {
            name: name.to_string(),
            nullable,
            scale,
        }
    }
}
