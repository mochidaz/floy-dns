use bcrypt::{hash, verify, DEFAULT_COST};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

use crate::common::errors::ErrorKind;
use crate::common::writers::Writer;
use crate::config::Config;
use crate::models::User;

pub fn validate_username(s: &String) -> bool {
    s.chars().all(|c| c.is_alphanumeric())
}

pub fn validate_email(s: &String) -> bool {
    s.contains('@')
}

pub fn hash_password(s: &String) -> String {
    hash(s, DEFAULT_COST).unwrap()
}

pub async fn send_verification_email(
    config: &Config,
    to: &String,
    token: &String,
) -> Result<(), ErrorKind> {
    let verification_url = format!("{}/api/verify?token={}", config.base_url, token);
    let email = lettre::Message::builder()
        .from(
            format!("{} <{}>", &config.smtp_from_name, &config.smtp_from_email)
                .parse()
                .unwrap(),
        )
        .to(to.parse().unwrap())
        .subject("Verifikasi FloyDNS")
        .body(format!(
            "Klik link berikut untuk memverifikasikan akun FloyDNS-mu:    {}",
            verification_url
        ))?;

    let creds = Credentials::new(
        config.smtp_username.to_owned(),
        config.smtp_password.to_owned(),
    );

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(config.smtp_host.as_str())
            ?
            .credentials(creds)
            .build();

    let m = mailer.send(email).await;

    Ok(())
}

pub async fn find_subdomain_claim(
    writer: &Writer<String>,
    subdomain_claim: &String,
) -> Result<Option<User>, ErrorKind> {
    writer
        .find(subdomain_claim)
        .await?
        .map(|s| Some(s))
        .ok_or(ErrorKind::NotFound)
}

#[cfg(test)]
mod tests {
    use bcrypt::verify;

    use super::*;

    #[test]
    fn test_validate_username() {
        let username_success = "username123".to_string();
        let username_fail = "username 123".to_string();

        assert_eq!(true, validate_username(&username_success));
        assert_eq!(false, validate_username(&username_fail));
    }

    #[test]
    fn test_hash_password() {
        let password = "password".to_string();
        let hashed_password = hash_password(&password);

        assert_ne!(password, hashed_password);
        assert_eq!(true, verify(&password, &hashed_password).unwrap());
    }
}
