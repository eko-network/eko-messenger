/// Serializes a single `String` as a single-item array `[String]` and deserializes back.
///
/// This is useful for ActivityPub fields like `to` which are typically arrays but we only
/// support single recipients.
pub mod single_item_vec {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    /// Deserialize a single-item array into a String
    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut v: Vec<String> = Vec::deserialize(deserializer)?;
        if v.len() != 1 {
            return Err(D::Error::custom(format!(
                "expected exactly 1 item in array, found {}",
                v.len()
            )));
        }
        Ok(v.remove(0))
    }

    /// Serialize a String as a single-item array
    pub fn serialize<S>(value: &String, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(std::iter::once(value))
    }
}

/// Serializes a borrowed `&str` as a single-item array `[String]` and deserializes to `String`.
///
/// This is useful for storage view structs that use `&'a str` references but need to
/// serialize the same way as owned `String` fields.
pub mod single_item_vec_borrowed {
    use serde::Serializer;
    pub fn serialize<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(std::iter::once(value))
    }
}
