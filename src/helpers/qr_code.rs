use regex::Regex;

#[derive(PartialEq)]
pub enum QrCodeResult {
    Valid(QrCode),
    Invalid(String),
}

#[derive(PartialEq)]
pub struct QrCode {
    pub qr_code_payload: String,
}

impl QrCode {
    pub fn new(qr_code_payload: String) -> QrCode {
        QrCode { qr_code_payload }
    }

    pub fn extract(&self) -> &str {
        let re = Regex::new(r".*secret=(.*?)(&.*)?$").unwrap();

        let secret = re
            .captures(self.qr_code_payload.as_str())
            .and_then(|cap| cap.get(1).map(|secret| secret.as_str()));

        secret.unwrap_or(self.qr_code_payload.as_str())
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
