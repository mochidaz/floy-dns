use std::env;
use std::ops::Deref;

use rocket::{Request, State};
use rocket::outcome::Outcome;
use rocket::request::FromRequest;

#[derive(Clone)]
pub struct Config {
    pub jwt_secret: String,
    pub smtp_host: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_email: String,
    pub smtp_from_name: String,
    pub base_url: String,
    pub cf_email: String,
    pub cf_api_key: String,
    pub cf_zone_id: String,
    pub dns_suffix: String,
    pub database_path: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            jwt_secret: env::var("JWT_SECRET").unwrap(),
            smtp_host: env::var("SMTP_HOST").unwrap(),
            smtp_username: env::var("SMTP_USERNAME").unwrap(),
            smtp_password: env::var("SMTP_PASSWORD").unwrap(),
            smtp_from_email: env::var("SMTP_FROM_EMAIL").unwrap(),
            smtp_from_name: env::var("SMTP_FROM_NAME").unwrap(),
            base_url: env::var("BASE_URL").unwrap(),
            cf_email: env::var("CF_EMAIL").unwrap(),
            cf_api_key: env::var("CF_API_KEY").unwrap(),
            cf_zone_id: env::var("CF_ZONE_ID").unwrap(),
            dns_suffix: env::var("DNS_SUFFIX").unwrap(),
            database_path: env::var("DATABASE_PATH").unwrap(),
        }
    }
}

pub struct ConfigGuard(pub Config);

impl<'a> From<&'a rocket::State<Config>> for Config {
    fn from(data: &'a State<Config>) -> Self {
        Self {
            jwt_secret: data.jwt_secret.clone(),
            smtp_host: data.smtp_host.clone(),
            smtp_username: data.smtp_username.clone(),
            smtp_password: data.smtp_password.clone(),
            smtp_from_email: data.smtp_from_email.clone(),
            smtp_from_name: data.smtp_from_name.clone(),
            base_url: data.base_url.clone(),
            cf_email: data.cf_email.clone(),
            cf_api_key: data.cf_api_key.clone(),
            cf_zone_id: data.cf_zone_id.clone(),
            dns_suffix: data.dns_suffix.clone(),
            database_path: data.database_path.clone(),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ConfigGuard {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let config = request
            .guard::<&rocket::State<Config>>()
            .await
            .succeeded()
            .unwrap();

        Outcome::Success(ConfigGuard(Config::from(config)))
    }
}

impl Deref for ConfigGuard {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}