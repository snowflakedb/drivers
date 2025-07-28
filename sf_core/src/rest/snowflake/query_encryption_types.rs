use serde::{Deserialize, Deserializer};

// Encryption material from Snowflake response (JSON parsing only)
#[derive(Debug, Deserialize)]
pub struct ExecResponseEncryptionMaterial {
    #[serde(rename = "queryStageMasterKey")]
    pub query_stage_master_key: String,
    #[serde(rename = "queryId")]
    pub query_id: String,
    #[serde(rename = "smkId")]
    pub smk_id: i64,
}

impl ExecResponseEncryptionMaterial {
    /// Convert ExecResponseEncryptionMaterial to EncryptionMaterial for the encryption module
    pub fn to_encryption_material(&self) -> crate::api_server::encryption::EncryptionMaterial {
        crate::api_server::encryption::EncryptionMaterial {
            query_stage_master_key: self.query_stage_master_key.clone(),
            query_id: self.query_id.clone(),
            smk_id: self.smk_id,
        }
    }
}

// Custom deserializer for encryption material that can handle both single object and array
pub fn deserialize_encryption_material<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<ExecResponseEncryptionMaterial>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, MapAccess, SeqAccess, Visitor};
    use std::fmt;

    struct EncryptionMaterialVisitor;

    impl<'de> Visitor<'de> for EncryptionMaterialVisitor {
        type Value = Option<Vec<ExecResponseEncryptionMaterial>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("encryption material as object, array, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_seq<S>(self, seq: S) -> Result<Self::Value, S::Error>
        where
            S: SeqAccess<'de>,
        {
            // It's an array - deserialize normally
            let materials: Vec<ExecResponseEncryptionMaterial> =
                Vec::deserialize(de::value::SeqAccessDeserializer::new(seq))?;
            Ok(Some(materials))
        }

        fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            // It's a single object - wrap in array
            let material: ExecResponseEncryptionMaterial =
                ExecResponseEncryptionMaterial::deserialize(
                    de::value::MapAccessDeserializer::new(map),
                )?;
            Ok(Some(vec![material]))
        }
    }

    deserializer.deserialize_any(EncryptionMaterialVisitor)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_encryption_material_single_object_deserialization() {
        let json = r#"{
            "src_locations": ["test_file.csv"],
            "stageInfo": {
                "location": "test-bucket/path/",
                "region": "us-east-1",
                "creds": {
                    "AWS_KEY_ID": "test_key_id",
                    "AWS_SECRET_KEY": "test_secret_key",
                    "AWS_TOKEN": "test_token"
                }
            },
            "encryptionMaterial": {
                "queryStageMasterKey": "dGVzdF9tYXN0ZXJfa2V5XzEyMzQ1Njc4OTBhYmNkZWY=",
                "queryId": "test-query-123",
                "smkId": 12345
            },
            "command": "UPLOAD"
        }"#;

        let data: crate::rest::snowflake::query_types::ExecResponseData =
            serde_json::from_str(json).unwrap();

        // Should successfully parse single object as array with one element
        assert!(data.encryption_material.is_some());
        let materials = data.encryption_material.unwrap();
        assert_eq!(materials.len(), 1);
        assert_eq!(materials[0].query_id, "test-query-123");
        assert_eq!(materials[0].smk_id, 12345);
    }

    #[test]
    fn test_encryption_material_array_deserialization() {
        let json = r#"{
            "src_locations": ["test_file.csv"],
            "stageInfo": {
                "location": "test-bucket/path/",
                "region": "us-east-1",
                "creds": {
                    "AWS_KEY_ID": "test_key_id",
                    "AWS_SECRET_KEY": "test_secret_key",
                    "AWS_TOKEN": "test_token"
                }
            },
            "encryptionMaterial": [
                {
                    "queryStageMasterKey": "dGVzdF9tYXN0ZXJfa2V5XzEyMzQ1Njc4OTBhYmNkZWY=",
                    "queryId": "test-query-123",
                    "smkId": 12345
                },
                {
                    "queryStageMasterKey": "YW5vdGhlcl90ZXN0X21hc3Rlcl9rZXlfZm9yX2ZpbGUy",
                    "queryId": "test-query-456",
                    "smkId": 67890
                }
            ],
            "command": "UPLOAD"
        }"#;

        let data: crate::rest::snowflake::query_types::ExecResponseData =
            serde_json::from_str(json).unwrap();

        // Should successfully parse array normally
        assert!(data.encryption_material.is_some());
        let materials = data.encryption_material.unwrap();
        assert_eq!(materials.len(), 2);
        assert_eq!(materials[0].query_id, "test-query-123");
        assert_eq!(materials[1].query_id, "test-query-456");
    }

    #[test]
    fn test_encryption_material_missing_field() {
        let json = r#"{
            "src_locations": ["test_file.csv"],
            "stageInfo": {
                "location": "test-bucket/path/",
                "region": "us-east-1",
                "creds": {
                    "AWS_KEY_ID": "test_key_id",
                    "AWS_SECRET_KEY": "test_secret_key",
                    "AWS_TOKEN": "test_token"
                }
            },
            "command": "UPLOAD"
        }"#;

        let data: crate::rest::snowflake::query_types::ExecResponseData =
            serde_json::from_str(json).unwrap();

        // Should successfully parse missing field as None
        assert!(data.encryption_material.is_none());
    }
}
