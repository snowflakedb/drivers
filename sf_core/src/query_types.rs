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
    Object {
        name: String,
        nullable: bool,
    },
    Array {
        name: String,
        nullable: bool,
    },
    Map {
        name: String,
        nullable: bool,
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

    pub fn map(name: &str, nullable: bool) -> Self {
        RowType::Map {
            name: name.to_string(),
            nullable,
        }
    }
}
