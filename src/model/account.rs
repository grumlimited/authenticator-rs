use std::time::SystemTime;

use base32::decode;
use base32::Alphabet::RFC4648;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Account {
    pub id: u32,
    pub group_id: u32,
    pub label: String,
    pub secret: String,
    totp: Option<String>,
}

impl Account {
    pub fn new(group_id: u32, label: &str, secret: &str) -> Self {
        let mut a = Account {
            group_id,
            label: label.to_owned(),
            secret: secret.to_owned(),
            ..Account::default()
        };

        a.update();
        a
    }

    pub fn generate_time_based_password(key: &str) -> Result<String, String> {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self::generate_time_based_password_with_time(time, key)
    }

    fn generate_time_based_password_with_time(time: u64, key: &str) -> Result<String, String> {
        if let Some(b32) = decode(RFC4648 { padding: false }, key) {
            let totp_sha1 = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, b32);
            totp_sha1.generate(time);
            Ok(totp_sha1.generate(time))
        } else {
            Err("error!".to_owned())
        }
    }

    pub fn update(&mut self) {
        match Self::generate_time_based_password(self.secret.as_str()) {
            Ok(totp) => self.totp = Some(totp),
            Err(_) => self.totp = None,
        }
    }
}
