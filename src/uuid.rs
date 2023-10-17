use std::fmt::{self, Display, Formatter};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A wrapper around a UUID (from `uuid::Uuid`) that serializes to a Base62 string.
///
/// Database rows have a UUID primary key, but they are encoded in Base62 to be shorter and more
/// URL-friendly for the frontend.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Base62Uuid(
    #[serde(deserialize_with = "uuid_from_base62_str")]
    #[serde(serialize_with = "uuid_to_base62_str")]
    Uuid,
);

impl Base62Uuid {
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for Base62Uuid {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for Base62Uuid {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Display for Base62Uuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base62_encode(self.0.as_u128()))
    }
}

impl TryFrom<&str> for Base62Uuid {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        Ok(Self(Uuid::from_u128(base62_decode(s)?)))
    }
}

impl From<Base62Uuid> for String {
    fn from(s: Base62Uuid) -> Self {
        base62_encode(s.0.as_u128())
    }
}

fn uuid_to_base62_str<S>(uuid: &Uuid, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&base62_encode(uuid.as_u128()))
}

fn uuid_from_base62_str<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(Uuid::from_u128(
        base62_decode(&s).map_err(serde::de::Error::custom)?,
    ))
}

pub fn base62_encode(mut number: u128) -> String {
    let base = BASE62_CHARS.len() as u128;
    let mut encoded = Vec::new();

    while number > 0 {
        let remainder = (number % base) as usize;
        number /= base;
        encoded.push(BASE62_CHARS[remainder]);
    }

    encoded.reverse();
    unsafe {
        // Safety: all characters in `encoded` must come from BASE62_CHARS, and characters in 
        // BASE62_CHARS are valid UTF-8 (they are ASCII). Therefore, `encoded` must contain only
        // valid UTF-8.
        String::from_utf8_unchecked(encoded)
    }
}

pub fn base62_decode(input: &str) -> Result<u128> {
    let base = BASE62_CHARS.len() as u128;
    let mut number = 0u128;

    if input.is_empty() {
        return Err(anyhow!("cannot decode an empty string"));
    }

    for &byte in input.as_bytes() {
        if let Some(value) = BASE62_CHARS.iter().position(|&ch| ch == byte) {
            number = number.checked_mul(base).context("u128 overflow")? + value as u128;
        }
    }

    Ok(number)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn test_encode_decode() {
        let original_uuids = [
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        for original_uuid in original_uuids.iter() {
            let encoded = base62_encode(original_uuid.as_u128());
            let decoded = base62_decode(&encoded).unwrap();
            let decoded_uuid = Uuid::from_u128(decoded);

            assert_eq!(*original_uuid, decoded_uuid);
        }
    }

    #[test]
    fn errors_if_encoded_string_has_extra_bytes() {
        let uuid = Uuid::new_v4();

        let encoded = base62_encode(uuid.as_u128());
        let encoded_plus_extra = format!("{}{}", encoded, "extra");
        let decode_result = base62_decode(&encoded_plus_extra);

        assert!(decode_result.is_err());
    }

    #[test]
    fn ignores_invalid_chars_in_encoded_string() {
        let uuid = Uuid::new_v4();

        let encoded = base62_encode(uuid.as_u128());
        let encoded_plus_invalid_chars = format!("!??{}", encoded);
        let decoded = base62_decode(&encoded_plus_invalid_chars).unwrap();
        let decoded_uuid = Uuid::from_u128(decoded);

        assert_eq!(uuid, decoded_uuid);
    }

    #[test]
    fn errors_if_encoded_string_is_empty() {
        let decode_result = base62_decode("");

        assert!(decode_result.is_err());
    }
}
