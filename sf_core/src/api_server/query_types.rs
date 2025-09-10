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
}

impl RowType {
    pub fn fixed(name: &str, nullable: bool, precision: u64, scale: u64) -> Result<Self, String> {
        if scale != 0 {
            return Err("Fixed types are supported only for scale equal to 0.".to_string());
        }
        Ok(RowType::Fixed {
            name: name.to_string(),
            nullable,
            precision,
            scale,
        })
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
}
