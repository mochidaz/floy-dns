use std::env;

use chrono::{Duration, Local};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::{request, Data, Request};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::errors::{ErrorKind, JWTCError};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Clone)]
pub struct ApiKey(pub String);

pub fn generate_token(config: &Config, key: &String) -> Result<String, ErrorKind> {
    let dt = Local::now();

    let now = dt.timestamp_nanos() as usize;

    let exp = now + (Duration::days(7).num_nanoseconds().unwrap() as usize);

    let mut sub = key.clone();

    let claims = Claims { sub, iat: now, exp };

    let header = Header::new(Algorithm::HS512);
    Ok(encode(
        &header,
        &claims,
        &EncodingKey::from_secret(&config.jwt_secret.as_bytes()),
    )?)
}

pub fn read_token(config: &Config, key: &str) -> Result<String, ErrorKind> {
    let dt = Local::now();

    let now = dt.timestamp_nanos() as usize;

    match decode::<Claims>(
        key,
        &DecodingKey::from_secret(&config.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS512),
    ) {
        Ok(v) => {
            if now > v.claims.exp {
                return Err(ErrorKind::JWTCreationError(JWTCError::TokenExpired));
            }
            Ok(v.claims.sub)
        }
        Err(e) => Err(ErrorKind::from(e)),
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey {
    type Error = ErrorKind;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<ApiKey, ErrorKind> {
        let keys = match request.headers().get_one("Authorization") {
            Some(k) => k.split("Bearer").map(|i| i.trim()).collect::<String>(),
            None => {
                return request::Outcome::Error((Status::BadRequest, ErrorKind::InvalidValue))
            }
        };

        let config = request.guard::<&rocket::State<Config>>().await.unwrap();

        match read_token(&config, keys.as_str()) {
            Ok(claim) => request::Outcome::Success(ApiKey(claim)),
            Err(e) => request::Outcome::Error((Status::Unauthorized, e)),
        }
    }
}
