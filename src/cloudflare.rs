use std::env;
use std::ops::Deref;

use reqwest::Client;
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::{json, Json};
use rocket::{request, Request};
use rocket_dyn_templates::handlebars::JsonValue;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;

use crate::config::{Config, ConfigGuard};
use crate::errors::ErrorKind;
use crate::models::{DnsRecord, Record, Records};

pub struct Cloudflare {
    client: Client,
    config: Config,
}

impl Cloudflare {
    pub async fn new(config: Config) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();

        headers.insert("X-Auth-Email", config.cf_email.parse().unwrap());
        headers.insert("X-Auth-Key", config.cf_api_key.parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let client = Client::builder().default_headers(headers).build().unwrap();

        Cloudflare { client, config }
    }

    pub async fn add_subdomain_dns_record(
        &self,
        subdomain: &String,
        ip: &String,
    ) -> Result<(), ErrorKind> {
        &self.get_subdomain_dns_record(subdomain, false).await?;

        let name = format!("{}.{}", subdomain, &self.config.dns_suffix);

        let client = &self.client;
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            &self.config.cf_zone_id
        );

        let body = DnsRecord::new("A".to_owned(), name.clone(), 1, ip.to_owned(), false);

        let res = client
            .post(&url)
            .bearer_auth(&self.config.cf_api_key)
            .body(body.to_string())
            .send()
            .await?;

        if res.status().is_success() {
        } else {
            return Err(ErrorKind::Error("Failed to add DNS record".to_string()));
        }

        let wildcard_body = DnsRecord::new("A".to_owned(), format!("*.{}", &name), 1, ip.to_owned(), false);

        let res = client
            .post(&url)
            .bearer_auth(&self.config.cf_api_key)
            .body(wildcard_body.to_string())
            .send()
            .await?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(ErrorKind::Error("Failed to add DNS record".to_string()))
        }
    }

    pub async fn get_subdomain_dns_record(
        &self,
        subdomain: &String,
        wildcard: bool,
    ) -> Result<String, ErrorKind> {
        let name = if wildcard {
            format!("*.{}", subdomain)
        } else {
            format!("{}.{}", subdomain, &self.config.dns_suffix)
        };

        let client = &self.client;

        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?type=A&name={}",
            &self.config.cf_zone_id, name
        );

        let res = client
            .get(&url)
            .bearer_auth(&self.config.cf_api_key)
            .send()
            .await?;

        if res.status().is_success() {
            let record = res.json::<Records>().await?;

            if record.result.len() < 1 {
                return Err(ErrorKind::Error("Failed to get DNS record".to_string()));
            }

            Ok(record.result[0].content.clone())
        } else {
            Err(ErrorKind::Error("Failed to get DNS record".to_string()))
        }
    }

    pub async fn update_subdomain_dns_record(
        &self,
        subdomain: &String,
        ip: &String,
    ) -> Result<(), ErrorKind> {
        let record = &self.get_subdomain_dns_record(subdomain, false).await?;

        let name = format!("{}.{}", subdomain, &self.config.dns_suffix);

        let client = &self.client;
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            &self.config.cf_zone_id
        );
        let body = DnsRecord::new("A".to_owned(), name.clone(), 1, ip.to_owned(), false);

        let res = client
            .put(&url)
            .bearer_auth(&self.config.cf_api_key)
            .body(body.to_string())
            .send()
            .await?;

        if res.status().is_success() {
        } else {
            return Err(ErrorKind::Error("Failed to update DNS record".to_string()));
        }

        let wildcard_record = &self.get_subdomain_dns_record(subdomain, true).await?;

        let wildcard_body = DnsRecord::new(
            "A".to_owned(),
            format!("*.{}", &name),
            1,
            ip.to_owned(),
            false,
        );

        let wildcard_res = client
            .put(&url)
            .bearer_auth(&self.config.cf_api_key)
            .body(wildcard_body.to_string())
            .send()
            .await?;

        if wildcard_res.status().is_success() {
            Ok(())
        } else {
            Err(ErrorKind::Error("Failed to update DNS record".to_string()))
        }
    }
}

pub struct CloudflareGuard(pub Cloudflare);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CloudflareGuard {
    type Error = ErrorKind;

    async fn from_request(request: &'r Request<'_>) -> Outcome<CloudflareGuard, ErrorKind> {
        let cloudflare = request.guard::<&rocket::State<Config>>().await.unwrap();

        Outcome::Success(CloudflareGuard(
            Cloudflare::new(Config::from(cloudflare)).await,
        ))
    }
}

impl Deref for CloudflareGuard {
    type Target = Cloudflare;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
