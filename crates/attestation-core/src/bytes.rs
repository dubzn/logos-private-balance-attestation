use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Digest32(pub [u8; 32]);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HexBytes(pub Vec<u8>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HexParseError {
    InvalidDigestLength { expected: usize, actual: usize },
    InvalidHex(String),
}

impl Digest32 {
    pub const ZERO: Self = Self([0; 32]);

    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(value: &str) -> Result<Self, HexParseError> {
        let bytes = decode_hex(value)?;
        if bytes.len() != 32 {
            return Err(HexParseError::InvalidDigestLength {
                expected: 32,
                actual: bytes.len(),
            });
        }
        let mut digest = [0_u8; 32];
        digest.copy_from_slice(&bytes);
        Ok(Self(digest))
    }
}

impl HexBytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    pub fn from_hex(value: &str) -> Result<Self, HexParseError> {
        Ok(Self(decode_hex(value)?))
    }
}

fn decode_hex(value: &str) -> Result<Vec<u8>, HexParseError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    hex::decode(value).map_err(|error| HexParseError::InvalidHex(error.to_string()))
}

impl From<[u8; 32]> for Digest32 {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl From<Digest32> for [u8; 32] {
    fn from(value: Digest32) -> Self {
        value.0
    }
}

impl From<Vec<u8>> for HexBytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<HexBytes> for Vec<u8> {
    fn from(value: HexBytes) -> Self {
        value.0
    }
}

impl fmt::Display for Digest32 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl fmt::Display for HexBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl fmt::Display for HexParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDigestLength { expected, actual } => {
                write!(
                    formatter,
                    "invalid digest length: expected {expected} bytes, got {actual}"
                )
            }
            Self::InvalidHex(error) => write!(formatter, "invalid hex: {error}"),
        }
    }
}

impl std::error::Error for HexParseError {}

impl FromStr for Digest32 {
    type Err = HexParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_hex(value)
    }
}

impl FromStr for HexBytes {
    type Err = HexParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_hex(value)
    }
}

impl Serialize for Digest32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Digest32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(serde::de::Error::custom)
    }
}

impl Serialize for HexBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for HexBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_serializes_as_lower_hex() {
        let digest = Digest32([0xab; 32]);
        let json = serde_json::to_string(&digest).unwrap();
        assert_eq!(
            json,
            "\"abababababababababababababababababababababababababababababababab\""
        );
        assert_eq!(serde_json::from_str::<Digest32>(&json).unwrap(), digest);
    }

    #[test]
    fn digest_rejects_wrong_length() {
        assert!(matches!(
            Digest32::from_hex("abcd"),
            Err(HexParseError::InvalidDigestLength {
                expected: 32,
                actual: 2
            })
        ));
    }
}
