use crate::helpers::QrCodeResult::{Invalid, Valid};
use log::warn;
use regex::Regex;
use rqrr::PreparedImage;

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
    pub fn extract(&self) -> &str {
        let re = Regex::new(r".*secret=(.*?)(&.*)?$").unwrap();

        let secret = re
            .captures(self.qr_code_payload.as_str())
            .and_then(|cap| cap.get(1).map(|secret| secret.as_str()));

        secret.unwrap_or(self.qr_code_payload.as_str())
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
    fn extract_secret_missing() {
        let qr_code_payload = "ABCD";
        let qr_code = QrCode::new(qr_code_payload.to_string());
        let result = qr_code.extract();
        assert_eq!("ABCD", result);
    }
}
