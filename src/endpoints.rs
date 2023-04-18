use bcrypt::verify;
use rocket::{Build, Rocket};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json::json;
use rocket_dyn_templates::handlebars::JsonValue;

use crate::cloudflare::CloudflareGuard;
use crate::config::{Config, ConfigGuard};
use crate::jwt::{ApiKey, generate_token, read_token};
use crate::models::{DNS, Login, User, WhoAmI};
use crate::utils::{find_subdomain_claim, hash_password, send_verification_email, validate_email, validate_username};
use crate::writers::{Writer, WriterConn};
use crate::errors::{ErrorKind};
use crate::errors::ErrorKind::Error;

#[post("/register", data = "<data>")]
async fn register(
    config: ConfigGuard,
    data: Json<User>,
    writer: WriterConn,
) -> Result<Json<JsonValue>, Status> {
    let user = data.into_inner();

    if let Some(w) = writer.find(&user.subdomain_claim).await.map_err(|_| Status::InternalServerError)? {
        return Err(Status::Conflict);
    }

    if let Some(s) = writer.find(&user.email).await.map_err(|_| Status::InternalServerError)? {
        return Err(Status::Conflict);
    }

    if !validate_username(&user.subdomain_claim) {
        return Err(Status::BadRequest);
    }

    if !validate_email(&user.email) {
        return Err(Status::BadRequest);
    }

    let hash = hash_password(&user.password);

    let message = format!("{}:{}:{}\r", &user.subdomain_claim, &user.email, &hash);

    let verification_token = generate_token(&config, &message).map_err(|_| Status::InternalServerError)?;

    match send_verification_email(&config, &user.email, &verification_token).await.map_err(|_| Status::InternalServerError) {
        Ok(_) => {
            {}
        },
        Err(_) => return Err(Status::InternalServerError)
    };

    Ok(Json(json!(
        {
            "status": 200,
            "message": "Cek emailmu untuk verifikasi akun!"
        }
    )))
}

#[post("/login", data = "<data>")]
async fn login(
    data: Json<Login>,
    writer: WriterConn,
    config: ConfigGuard,
) -> Result<Json<JsonValue>, Status> {
    let user = data.into_inner();

    let search = match writer.find(&user.email).await.map_err(|_| Status::InternalServerError)? {
        Some(search) => search,
        None => return Err(Status::NotFound),
    };

    let password = &search.password;

    if !verify(&user.password, &password).map_err(|_| Status::InternalServerError)? {
        return Err(Status::Unauthorized);
    }

    let token = generate_token(&config, &user.email).map_err(|_| Status::InternalServerError)?;

    Ok(Json(json!(
        {
            "status": 200,
            "message": "User logged in successfully",
            "token": token,
        }
    )))
}

#[get("/whoami")]
async fn whoami(writer: WriterConn, key: ApiKey) -> Result<Json<JsonValue>, Status> {
    let user = match writer.find(&key.0).await {
        Ok(user) => match user {
            Some(user) => user,
            None => return Err(Status::NotFound)
        },

        Err(_) => return Err(Status::InternalServerError)
    };

    Ok(Json(json!(
        {
            "status": 200,
            "message": "User found",
            "data": WhoAmI {
                subdomain_claim: user.subdomain_claim,
                email: user.email,
            }
        }
    )))
}

#[get("/verify?<token>")]
async fn verify_account(config: ConfigGuard, token: Option<String>, writer: WriterConn) -> Result<Json<JsonValue>, Status> {
    let token = match token {
        Some(token) => token,
        None => return Err(Status::BadRequest)
    };

    let read = match read_token(&config, &token).map_err(|_| Status::InternalServerError) {
        Ok(read) => read,
        Err(_) => return Err(Status::Unauthorized)
    };

    writer.write(&read).await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(json!(
        {
            "status": 200,
            "message": "Account verified. Please log in to continue",
            "verified": true,
        }
    )))
}

#[get("/dns")]
async fn dns(config: ConfigGuard, cloudflare: CloudflareGuard, writer: WriterConn, key: ApiKey) -> Result<Json<JsonValue>, Status> {
    let user = match find_subdomain_claim(&writer, &key.0).await.map_err(|_| Status::InternalServerError) {
        Ok(subdomain_claim) => match subdomain_claim {
            Some(subdomain_claim) => subdomain_claim,
            None => return Err(Status::NotFound)
        },
        Err(_) => return Err(Status::InternalServerError)
    };

    let dns_entry = match cloudflare.get_subdomain_dns_record(&user.subdomain_claim, false).await.map_err(|_| Status::InternalServerError) {
        Ok(dns_entry) => dns_entry,
        Err(_) => return Err(Status::NotFound)
    };

    Ok(Json(json!(
        {
            "status": 200,
            "message": "DNS record found",
            "ip": dns_entry,
        }
    )))
}

#[post("/dns", data = "<dns>")]
async fn dns_add(config: ConfigGuard, cloudflare: CloudflareGuard, writer: WriterConn, key: ApiKey, dns: Json<DNS>) -> Result<Json<JsonValue>, Status> {
    let user = match find_subdomain_claim(&writer, &key.0).await.map_err(|_| Status::InternalServerError) {
        Ok(subdomain_claim) => match subdomain_claim {
            Some(subdomain_claim) => subdomain_claim,
            None => return Err(Status::NotFound)
        },
        Err(_) => return Err(Status::InternalServerError)
    };

    match cloudflare.add_subdomain_dns_record(&user.subdomain_claim, &dns.ip).await.map_err(|_| Status::InternalServerError) {
        Ok(_) => {},
        Err(_) => return Err(Status::Conflict)
    };

    Ok(Json(json!(
        {
            "status": 200,
            "message": "DNS record added",
            "ip": &dns.ip,
        }
    )))
}

#[put("/dns", data = "<dns>")]
async fn dns_update(config: ConfigGuard, cloudflare: CloudflareGuard, writer: WriterConn, key: ApiKey, dns: Json<DNS>) -> Result<Json<JsonValue>, Status> {
    let user = match find_subdomain_claim(&writer, &key.0).await.map_err(|_| Status::InternalServerError) {
        Ok(subdomain_claim) => match subdomain_claim {
            Some(subdomain_claim) => subdomain_claim,
            None => return Err(Status::NotFound)
        },
        Err(_) => return Err(Status::InternalServerError)
    };

    match cloudflare.update_subdomain_dns_record(&user.subdomain_claim, &dns.ip).await.map_err(|_| Status::InternalServerError) {
        Ok(_) => {},
        Err(_) => return Err(Status::Conflict)
    };

    Ok(Json(json!(
        {
            "status": 200,
            "message": "DNS record updated",
            "ip": &dns.ip,
        }
    )))
}

pub async fn build_endpoints() -> Rocket<Build> {
    rocket::build()
        .mount(
            "/api",
            routes![
                register,
                login,
                whoami,
                dns,
                dns_add,
                dns_update,
                verify_account,
            ],
        )
}
