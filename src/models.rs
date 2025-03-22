use std::fmt;

use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub email: String,
    pub password: String,
    pub subdomain_claim: String,
}

#[derive(Serialize, Deserialize)]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct WhoAmI {
    pub email: String,
    pub subdomain_claim: String,
}

#[derive(Serialize, Deserialize)]
pub struct DNS {
    pub ip: String,
    pub subdomain: String,
    pub web_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct Records {
    pub result: Vec<Record>,
}

#[derive(Serialize)]
pub struct DnsRecord {
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub ttl: u32,
    pub content: String,
    pub proxied: bool,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SubdomainRequest {
    pub user_id: String,
    pub business_id: String,
    pub subdomain: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SlugRequest {
    pub user_id: String,
    pub business_id: String,
    pub slug: String,
    pub previous_slug: String,
    pub site_id: String,
    pub rewrite_target: Option<String>,
    pub subdomain: String,
}

impl DnsRecord {
    pub fn new(
        record_type: String,
        name: String,
        ttl: u32,
        content: String,
        proxied: bool,
    ) -> Self {
        DnsRecord {
            record_type,
            name,
            ttl,
            content,
            proxied,
        }
    }
}

impl fmt::Display for DnsRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"type\":\"{}\",\"name\":\"{}\",\"ttl\":{},\"content\":\"{}\",\"proxied\":{}}}",
            self.record_type, self.name, self.ttl, self.content, self.proxied
        )
    }
}
