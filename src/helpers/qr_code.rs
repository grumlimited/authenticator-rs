use crate::helpers::QrCodeResult::{Invalid, Valid};
use log::warn;
use percent_encoding::percent_decode_str;
use rqrr::PreparedImage;
use url::Url;

#[derive(PartialEq, Debug)]
pub enum QrCodeResult {
    Valid(QrCode),
    Invalid(String),
}

#[derive(PartialEq, Debug)]
pub struct QrCode {
    pub qr_code_payload: String,
}

impl QrCode {
    pub fn new(qr_code_payload: String) -> QrCode {
        QrCode { qr_code_payload }
    }

    /// Extract the `secret` parameter from the payload if present,
    /// otherwise return the full payload unchanged.
    pub fn extract(&self) -> String {
        // Try to parse as a URL and extract the 'secret' query parameter (handles percent-encoding)
        if let Ok(url) = Url::parse(self.qr_code_payload.as_str()) {
            for (k, v) in url.query_pairs() {
                if k.eq_ignore_ascii_case("secret") {
                    // percent-decode the value and return it as owned String
                    let decoded = percent_decode_str(&v).decode_utf8_lossy().into_owned();
                    return decoded;
                }
            }
        } else {
            // Not a valid URL, fall back to query-like parsing
            if let Some(idx) = self.qr_code_payload.find("secret=") {
                let after = &self.qr_code_payload[idx + "secret=".len()..];
                // secret may be terminated by & or end of string
                let end = after.find('&').unwrap_or(after.len());
                let candidate = &after[..end];
                // percent-decode candidate into an owned String and return
                let decoded = percent_decode_str(candidate).decode_utf8_lossy().into_owned();
                return decoded;
            }
        }

        // Default: return the full payload as owned String
        self.qr_code_payload.clone()
    }

    /// Process an image file at `path` and attempt to decode a QR code.
    /// Returns `Valid(QrCode)` on success or `Invalid(String)` with a
    /// descriptive message on failure.
    pub async fn process_qr_code(path: String) -> QrCodeResult {
        match image::open(&path) {
            Ok(img) => {
                let luma_img = img.to_luma8();
                let mut prepared = PreparedImage::prepare(luma_img);
                let grids = prepared.detect_grids();

                match grids.len() {
                    0 => {
                        warn!("No QR grids found in {}", path);
                        Invalid(format!("No QR codes found in {}", path))
                    }
                    n => {
                        if n > 1 {
                            warn!("Multiple QR grids found in {}, attempting first", path);
                        }

                        match grids[0].decode() {
                            Ok((_, content)) => Valid(QrCode::new(content)),
                            Err(e) => {
                                warn!("Failed to decode QR from {}: {}", path, e);
                                Invalid(format!("Failed to decode QR code: {}", e))
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to open image {}: {}", path, e);
                Invalid(format!("Failed to open image: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::QrCode;

    #[test]
    fn extract_secret_end() {
        let qr_code_payload = "otpauth://totp/Soomesite:nonfunc@gmail.com?algorithm=SHA1&digits=6&issuer=Somesite&period=30&secret=ABCD";
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_middle() {
        let qr_code_payload = "otpauth://totp/Soomesite:nonfunc@gmail.com?algorithm=SHA1&digits=6&secret=ABCD&issuer=Somesite&period=30";
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_beginning() {
        let qr_code_payload = "secret=ABCD&otpauth://totp/Soomesite:nonfunc@gmail.com?algorithm=SHA1&digits=6&secret=ABCD&issuer=Somesite&period=30"; //non sensical
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_by_itself() {
        let qr_code_payload = "secret=ABCD";
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_with_ampersands() {
        let qr_code_payload = "&secret=ABCD&";
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_percent_encoded_secret_in_otpauth_uri() {
        let qr_code_payload = "otpauth://totp/Example:alice?secret=ABC%2BDEF&issuer=Example";
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("ABC+DEF", result);
    }

    #[test]
    fn extract_first_of_multiple_secret_params() {
        let qr_code_payload = "otpauth://totp/Example:alice?issuer=Ex&secret=FIRST&foo=bar&secret=SECOND";
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("FIRST", result);
    }

    #[test]
    fn extract_percent_encoded_secret_in_plain_query() {
        let qr_code_payload = "foo=bar&secret=XYZ%252B123&baz=1"; // note double-encoding scenario
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        // percent-decode of "XYZ%252B123" -> "XYZ%2B123" (decode once), we expect that decoding path returns the single-decode result
        assert_eq!("XYZ%2B123", result);
    }

    #[test]
    fn extract_secret_missing() {
        let qr_code_payload = "ABCD";
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("ABCD", result);
    }
}
