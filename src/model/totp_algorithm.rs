use serde::{de::Visitor, Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use std::fmt;
use totp_rs::Algorithm;

/// Newtype wrapper for Algorithm that implements Serialize/Deserialize and From<> traits
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TotpAlgorithm(pub Algorithm);

impl Default for TotpAlgorithm {
    fn default() -> Self {
        TotpAlgorithm(Algorithm::SHA1)
    }
}

impl TotpAlgorithm {
    /// Parse a TotpAlgorithm from a string
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "SHA1" => TotpAlgorithm(Algorithm::SHA1),
            "SHA256" => TotpAlgorithm(Algorithm::SHA256),
            "SHA512" => TotpAlgorithm(Algorithm::SHA512),
            "STEAM" => TotpAlgorithm(Algorithm::Steam),
            _ => TotpAlgorithm(Algorithm::SHA1),
        }
    }

    /// Convert to string slice
    pub fn as_str(&self) -> &'static str {
        match self.0 {
            Algorithm::SHA1 => "SHA1",
            Algorithm::SHA256 => "SHA256",
            Algorithm::SHA512 => "SHA512",
            Algorithm::Steam => "STEAM",
            _ => "SHA1", // Non-exhaustive enum in totp-rs v6, default to SHA1
        }
    }
}

// Implement Serialize for TotpAlgorithm
impl Serialize for TotpAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

// Implement Deserialize for TotpAlgorithm
impl<'de> Deserialize<'de> for TotpAlgorithm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AlgorithmVisitor;

        impl<'de> Visitor<'de> for AlgorithmVisitor {
            type Value = TotpAlgorithm;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing SHA1, SHA256, or SHA512")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TotpAlgorithm::parse(value))
            }
        }

        deserializer.deserialize_str(AlgorithmVisitor)
    }
}

// Implement From<Algorithm> for TotpAlgorithm
impl From<Algorithm> for TotpAlgorithm {
    fn from(algorithm: Algorithm) -> Self {
        TotpAlgorithm(algorithm)
    }
}

// Implement From<TotpAlgorithm> for Algorithm
impl From<TotpAlgorithm> for Algorithm {
    fn from(algorithm: TotpAlgorithm) -> Self {
        algorithm.0
    }
}

// Implement From<&str> for TotpAlgorithm
impl From<&str> for TotpAlgorithm {
    fn from(s: &str) -> Self {
        TotpAlgorithm::parse(s)
    }
}

// Implement From<String> for TotpAlgorithm
impl From<String> for TotpAlgorithm {
    fn from(s: String) -> Self {
        TotpAlgorithm::parse(&s)
    }
}
