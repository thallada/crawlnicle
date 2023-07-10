use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A wrapper around a UUID (from `uuid::Uuid`) that serializes to a Base62 string.
///
/// Database rows have a UUID primary key, but they are encoded in Base62 to be shorter and more 
/// URL-friendly for the frontend.
#[derive(Debug, Serialize, Deserialize)]
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

impl From<&str> for Base62Uuid {
    fn from(s: &str) -> Self {
        Self(Uuid::from_u128(base62_decode(s)))
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
    Ok(Uuid::from_u128(base62_decode(&s)))
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
    String::from_utf8(encoded).unwrap()
}

pub fn base62_decode(input: &str) -> u128 {
    let base = BASE62_CHARS.len() as u128;
    let mut number = 0u128;

    for &byte in input.as_bytes() {
        number = number * base + (BASE62_CHARS.iter().position(|&ch| ch == byte).unwrap() as u128);
    }

    number
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
            let decoded_uuid = Uuid::from_u128(base62_decode(&encoded));

            assert_eq!(*original_uuid, decoded_uuid);
        }
    }
}
