use std::fmt;

use lettre;
use reqwest;
use rocket::futures::io::Cursor;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{Request, Response};
use rocket_dyn_templates::handlebars::JsonValue;

#[derive(Debug)]
pub enum JWTCError {
    TokenExpired,
}

#[derive(Serialize, Deserialize)]
struct Catcher {
    status: u16,
    message: String,
}

#[derive(Debug)]
pub enum ErrorKind {
    IOError(std::io::Error),
    JWTError(jsonwebtoken::errors::Error),
    JWTCreationError(JWTCError),
    InvalidValue,
    NotFound,
    EmailAlreadyExists,
    UsernameAlreadyExists,
    Error(String),
    ReqwestError(reqwest::Error),
    LettreTransportError(lettre::transport::smtp::Error),
    LettreError(lettre::error::Error),
}

impl From<std::io::Error> for ErrorKind {
    fn from(error: std::io::Error) -> Self {
        ErrorKind::IOError(error)
    }
}

impl From<jsonwebtoken::errors::Error> for ErrorKind {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        ErrorKind::JWTError(error)
    }
}

impl From<JWTCError> for ErrorKind {
    fn from(error: JWTCError) -> Self {
        ErrorKind::JWTCreationError(error)
    }
}

impl From<reqwest::Error> for ErrorKind {
    fn from(error: reqwest::Error) -> Self {
        ErrorKind::ReqwestError(error)
    }
}

impl From<lettre::transport::smtp::Error> for ErrorKind {
    fn from(error: lettre::transport::smtp::Error) -> Self {
        ErrorKind::LettreTransportError(error)
    }
}

impl From<lettre::error::Error> for ErrorKind {
    fn from(error: lettre::error::Error) -> Self {
        ErrorKind::LettreError(error)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ErrorKind::IOError(err) => err.to_string(),
            ErrorKind::JWTError(err) => err.to_string(),
            ErrorKind::JWTCreationError(err) => match err {
                JWTCError::TokenExpired => "Token expired".to_string(),
            },
            ErrorKind::InvalidValue => "Invalid value".to_string(),
            ErrorKind::NotFound => "Not found".to_string(),
            ErrorKind::EmailAlreadyExists => "Email already exists".to_string(),
            ErrorKind::UsernameAlreadyExists => "Username already exists".to_string(),
            ErrorKind::Error(err) => err.to_string(),
            ErrorKind::ReqwestError(err) => err.to_string(),
            ErrorKind::LettreTransportError(err) => err.to_string(),
            ErrorKind::LettreError(err) => err.to_string(),
        };

        write!(f, "{}", msg)
    }
}

#[catch(400)]
fn bad_request() -> Json<Catcher> {
    Catcher {
        status: 400,
        message: "Bad request".to_string(),
    }
    .into()
}

#[catch(401)]
fn unauthorized() -> Json<Catcher> {
    Catcher {
        status: 401,
        message: "Unauthorized".to_string(),
    }
    .into()
}

#[catch(404)]
fn not_found() -> Json<Catcher> {
    Catcher {
        status: 404,
        message: "Not found".to_string(),
    }
    .into()
}

#[catch(409)]
fn conflict() -> Json<Catcher> {
    Catcher {
        status: 409,
        message: "Conflict".to_string(),
    }
    .into()
}

#[catch(500)]
fn internal_server_error() -> Json<Catcher> {
    Catcher {
        status: 500,
        message: "Internal server error".to_string(),
    }
    .into()
}

pub async fn build_catchers() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Catcher", |rocket| async move {
        rocket.register(
            "/",
            catchers![
                bad_request,
                unauthorized,
                not_found,
                conflict,
                internal_server_error
            ],
        )
    })
}
