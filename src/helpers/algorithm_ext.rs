use totp_rs::Algorithm;

/// Extension trait for Algorithm to provide string conversion utilities
pub trait AlgorithmExt {
    /// Convert algorithm to string representation
    fn as_str(&self) -> &'static str;

    /// Convert algorithm to owned String
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }

    /// Parse algorithm from string, defaults to SHA1
    fn parse(s: &str) -> Algorithm;
}

impl AlgorithmExt for Algorithm {
    fn as_str(&self) -> &'static str {
        match self {
            Algorithm::SHA1 => "SHA1",
            Algorithm::SHA256 => "SHA256",
            Algorithm::SHA512 => "SHA512",
        }
    }

    fn parse(s: &str) -> Algorithm {
        match s.to_uppercase().as_str() {
            "SHA1" => Algorithm::SHA1,
            "SHA256" => Algorithm::SHA256,
            "SHA512" => Algorithm::SHA512,
            _ => Algorithm::SHA1,
        }
    }
}
