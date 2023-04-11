use chrono::{Duration, Local};
use std::env;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rocket::{Data, Request, request};
use rocket::http::Status;
use rocket::request::FromRequest;

use crate::errors::{ErrorKind, JWTCError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Clone)]
pub struct ApiKey(pub String);

pub fn generate_token(key: &String) -> Result<String> {
    let dt = Local::now();

    let now = dt.timestamp_nanos() as usize;

    let exp = now + (Duration::days(7).num_nanoseconds().unwrap() as usize);

    let mut sub = key.clone();

    let claims = Claims {
        sub,
        iat: now,
        exp,
    };

    let header = Header::new(Algorithm::HS512);
    Ok(encode(
        &header,
        &claims,
        &EncodingKey::from_secret(env::var("SECRETS").unwrap().as_bytes()),
    )?)
}

pub fn read_token(key: &str) -> Result<String> {
    let dt = Local::now();

    let now = dt.timestamp_nanos() as usize;

    match decode::<Claims>(
        key,
        &DecodingKey::from_secret(env::var("SECRETS").unwrap().as_bytes().as_ref()),
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
        let keys = match request
            .headers()
            .get_one("Authorization")
        {
            Some(k) => k.split("Bearer").map(|i| i.trim()).collect::<String>(),
            None => {
                return request::Outcome::Failure((Status::BadRequest, ErrorKind::InvalidValue))
            }
        };

        match read_token(keys.as_str()) {
            Ok(claim) => request::Outcome::Success(ApiKey(claim)),
            Err(e) => request::Outcome::Failure((Status::Unauthorized, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        env::set_var("SECRETS", "test");
        let key = "username";
        let token = generate_token(&key.to_string()).unwrap();
        assert_eq!(read_token(token.as_str()).unwrap(), key);
        env::remove_var("SECRETS");
    }
}
