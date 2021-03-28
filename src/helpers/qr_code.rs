use regex::Regex;

pub struct QrCode;

impl QrCode {
    pub fn extract(qr_code_payload: &str) -> &str {
        let re = Regex::new(r".*secret=(.*?)(&.*)?$").unwrap();

        let secret = re.captures(qr_code_payload).and_then(|cap| cap.get(1).map(|secret| secret.as_str()));

        match secret {
            Some(v) => v,
            None => qr_code_payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::QrCode;

    #[test]
    fn extract_secret_end() {
        let qr_code_payload = "otpauth://totp/Soomesite:nonfunc@gmail.com?algorithm=SHA1&digits=6&issuer=Somesite&period=30&secret=ABCD";
        let result = QrCode::extract(qr_code_payload);
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_middle() {
        let qr_code_payload = "otpauth://totp/Soomesite:nonfunc@gmail.com?algorithm=SHA1&digits=6&secret=ABCD&issuer=Somesite&period=30";
        let result = QrCode::extract(qr_code_payload);
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_beginning() {
        let qr_code_payload = "secret=ABCD&otpauth://totp/Soomesite:nonfunc@gmail.com?algorithm=SHA1&digits=6&secret=ABCD&issuer=Somesite&period=30"; //non sensical
        let result = QrCode::extract(qr_code_payload);
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_by_itself() {
        let qr_code_payload = "secret=ABCD";
        let result = QrCode::extract(qr_code_payload);
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_with_ampersands() {
        let qr_code_payload = "&secret=ABCD&";
        let result = QrCode::extract(qr_code_payload);
        assert_eq!("ABCD", result);
    }

    #[test]
    fn extract_secret_missing() {
        let qr_code_payload = "ABCD";
        let result = QrCode::extract(qr_code_payload);
        assert_eq!("ABCD", result);
    }
}
