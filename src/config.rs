use std::env;

#[derive(Clone)]
pub struct Config {
    pub jwt_secret: String,
    pub smtp_host: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_email: String,
    pub smtp_from_name: String,
    pub base_url: String,
    pub prefix: String,
    pub cf_email: String,
    pub cf_api_key: String,
    pub cf_zone_id: String,
    pub dns_suffix: String,
    pub database_path: String,
    pub ip: String,
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
            prefix: env::var("PREFIX").unwrap(),
            ip: env::var("IP").unwrap(),
        }
    }
}
