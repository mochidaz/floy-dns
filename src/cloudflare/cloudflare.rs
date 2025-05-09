use reqwest::Client;
use rocket::request::FromRequest;

use crate::common::errors::ErrorKind;
use crate::config::Config;
use crate::models::{DnsRecord, Records};

pub struct Cloudflare {
    client: Client,
    config: Config,
}

impl Cloudflare {
    pub async fn new(config: Config) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();

        // headers.insert("X-Auth-Email", config.cf_email.parse().unwrap());
        // headers.insert("X-Auth-Key", config.cf_api_key.parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", config.cf_api_key).parse().unwrap(),
        );

        let client = Client::builder().default_headers(headers).build().unwrap();

        Cloudflare { client, config }
    }

    pub async fn add_subdomain_dns_record(
        &self,
        subdomain: &String,
        ip: &String,
    ) -> Result<(), ErrorKind> {
        match &self.get_subdomain_dns_record(subdomain, false).await {
            Ok(_) => {
                return Err(ErrorKind::Error("Subdomain already exists".to_string()));
            }
            Err(_) => {}
        };

        let name = format!("{}.{}", subdomain, &self.config.dns_suffix);

        let client = &self.client;
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            &self.config.cf_zone_id
        );

        let body = DnsRecord::new("A".to_owned(), name.clone(), 1, ip.to_owned(), true);

        let res = client
            .post(&url)
            .bearer_auth(&self.config.cf_api_key)
            .body(body.to_string())
            .send()
            .await?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(ErrorKind::Error("Failed to add DNS record".to_string()))
        }

        // let wildcard_body = DnsRecord::new(
        //     "A".to_owned(),
        //     format!("*.{}", &name),
        //     1,
        //     ip.to_owned(),
        //     true,
        // );
        //
        // let res = client
        //     .post(&url)
        //     .bearer_auth(&self.config.cf_api_key)
        //     .body(wildcard_body.to_string())
        //     .send()
        //     .await?;
        //
        // if res.status().is_success() {
        //     Ok(())
        // } else {
        //     Err(ErrorKind::Error("Failed to add DNS record".to_string()))
        // }
    }

    pub async fn get_subdomain_dns_record(
        &self,
        subdomain: &String,
        wildcard: bool,
    ) -> Result<(String, String), ErrorKind> {
        let name = if wildcard {
            format!("*.{}.{}", subdomain, &self.config.dns_suffix)
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

            Ok((
                record.result[0].content.clone(),
                record.result[0].id.clone(),
            ))
        } else {
            Err(ErrorKind::Error("Failed to get DNS record".to_string()))
        }
    }

    pub async fn update_subdomain_dns_record(
        &self,
        subdomain: &String,
        ip: &String,
    ) -> Result<(), ErrorKind> {
        let record = match &self.get_subdomain_dns_record(subdomain, false).await {
            Ok(r) => r.clone(),
            Err(_) => {
                return Err(ErrorKind::Error("Subdomain does not exist".to_string()));
            }
        };

        let name = format!("{}.{}", subdomain, &self.config.dns_suffix);

        let client = &self.client;
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            &self.config.cf_zone_id
        );

        let body = DnsRecord::new("A".to_owned(), name.clone(), 1, ip.to_owned(), true);

        let res = client
            .put(format!("{}/{}", &url, record.1))
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
            true,
        );

        let wildcard_res = client
            .put(&format!("{}/{}", &url, wildcard_record.1))
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

    pub async fn delete_subdomain_dns_record(&self, subdomain: &String) -> Result<(), ErrorKind> {
        let record = match &self.get_subdomain_dns_record(subdomain, false).await {
            Ok(r) => r.clone(),
            Err(_) => {
                return Err(ErrorKind::Error("Subdomain does not exist".to_string()));
            }
        };

        let client = &self.client;
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            &self.config.cf_zone_id
        );

        let res = client
            .delete(format!("{}/{}", &url, record.1))
            .bearer_auth(&self.config.cf_api_key)
            .send()
            .await?;

        if res.status().is_success() {
        } else {
            return Err(ErrorKind::Error("Failed to delete DNS record".to_string()));
        }

        let wildcard_record = &self.get_subdomain_dns_record(subdomain, true).await?;

        let wildcard_res = client
            .delete(&format!("{}/{}", &url, wildcard_record.1))
            .bearer_auth(&self.config.cf_api_key)
            .send()
            .await?;

        if wildcard_res.status().is_success() {
            Ok(())
        } else {
            Err(ErrorKind::Error("Failed to delete DNS record".to_string()))
        }
    }

    pub async fn check_exists(&self, subdomain: &String) -> Result<bool, ErrorKind> {
        let name = format!("{}.{}", subdomain, &self.config.dns_suffix);

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
                return Ok(false);
            }

            Ok(true)
        } else {
            Err(ErrorKind::Error("Failed to get DNS record".to_string()))
        }
    }
}
