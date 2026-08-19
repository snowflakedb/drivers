use napi_derive::napi;
use sf_core::apis::database_driver_v1::ColumnMetadata;

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
    pub(crate) fn from_metadata(index: u32, meta: &ColumnMetadata) -> Self {
        Self {
            name: meta.name.clone(),
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
}
