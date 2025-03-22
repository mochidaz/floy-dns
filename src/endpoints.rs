use bcrypt::verify;
use reqwest::{get, redirect};
use rocket::http::Status;
use rocket::log::private::{log, logger, Level, Log};
use rocket::response::Redirect;
use rocket::serde::json::serde_json::json;
use rocket::serde::json::Json;
use rocket::{Build, Rocket, State};
use rocket_dyn_templates::handlebars::JsonValue;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

use crate::cloudflare::cloudflare::Cloudflare;
use crate::common::errors::ErrorKind;
use crate::common::errors::ErrorKind::Error;
use crate::common::jwt::{generate_token, read_token, ApiKey};
use crate::common::utils::{
    find_subdomain_claim, hash_password, send_verification_email, validate_email, validate_username,
};
use crate::common::writers::Writer;
use crate::config::Config;
use crate::models::{SubdomainRequest, Login, SlugRequest, User, WhoAmI, DNS};
use crate::updater::updater;

#[post("/register", data = "<data>")]
async fn register(
    config: &State<Config>,
    data: Json<User>,
    writer: &State<Writer<String>>,
) -> Result<Json<JsonValue>, Status> {
    let user = data.into_inner();

    if let Some(w) = writer
        .find(&user.subdomain_claim)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Conflict);
    }

    if let Some(s) = writer
        .find(&user.email)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
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

    let verification_token =
        generate_token(&config, &message).map_err(|_| Status::InternalServerError)?;

    match send_verification_email(&config, &user.email, &verification_token)
        .await
        .map_err(|_| Status::InternalServerError)
    {
        Ok(_) => {}
        Err(_) => return Err(Status::InternalServerError),
    };

    Ok(Json(json!(
        {
            "status": 200,
            "message": "Cek emailmu untuk verifikasi akun!"
        }
    )))
}

#[post("/auth", data = "<data>")]
async fn auth(
    data: Json<Login>,
    writer: &State<Writer<String>>,
    config: &State<Config>,
) -> Result<Json<JsonValue>, Status> {
    let user = data.into_inner();

    let search = match writer
        .find(&user.email)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
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
async fn whoami(writer: &State<Writer<String>>, key: ApiKey) -> Result<Json<JsonValue>, Status> {
    let user = match writer.find(&key.0).await {
        Ok(user) => match user {
            Some(user) => user,
            None => return Err(Status::NotFound),
        },

        Err(_) => return Err(Status::InternalServerError),
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
async fn verify_account(
    config: &State<Config>,
    token: Option<String>,
    writer: &State<Writer<String>>,
) -> Result<Redirect, Status> {
    let token = match token {
        Some(token) => token,
        None => return Err(Status::BadRequest),
    };

    let read = match read_token(&config, &token).map_err(|_| Status::InternalServerError) {
        Ok(read) => read,
        Err(_) => return Err(Status::Unauthorized),
    };

    let get_user = read.split(":").collect::<Vec<&str>>()[1];

    if let Some(user) = writer
        .find(get_user)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Conflict);
    }

    writer
        .write(&read)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Redirect::to("/login?verified=true"))
}

#[post("/domain", data = "<req>")]
pub async fn create_domain_endpoint(
    req: Json<SubdomainRequest>,
    cfg: &State<Config>,
    cloudflare: &State<Cloudflare>,
) -> Result<Json<JsonValue>, Status> {
    if (cloudflare
        .check_exists(&req.subdomain)
        .await
        .map_err(|_| Status::InternalServerError)?)
    {
        return Err(Status::Conflict);
    }

    cloudflare
        .add_subdomain_dns_record(&req.subdomain, &cfg.ip)
        .await
        .map_err(|_| Status::InternalServerError)?;

    updater::create_domain(
        &req.user_id,
        &req.business_id,
        &req.subdomain,
        &cfg,
    )
    .map_err(|_| Status::InternalServerError)?;
    Ok(Json(json!({
        "status": 200,
        "message": "Domain created successfully",
        "domain": format!("{}.{}", req.subdomain, cfg.dns_suffix)
    })))
}

#[delete("/domain", data = "<req>")]
pub async fn delete_domain_endpoint(
    req: Json<SubdomainRequest>,
    cfg: &State<Config>,
    cloudflare: &State<Cloudflare>,
) -> Result<Json<JsonValue>, Status> {
    if !(cloudflare
        .check_exists(&req.subdomain)
        .await
        .map_err(|_| Status::InternalServerError)?)
    {
        return Err(Status::NotFound);
    }

    match cloudflare.delete_subdomain_dns_record(&req.subdomain).await {
        Ok(_) => {}
        Err(_) => return Err(Status::InternalServerError),
    }

    updater::delete_domain(&req.user_id, &req.business_id)
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(json!({
        "status": 200,
        "message": "Domain deleted successfully"
    })))
}

#[post("/slug", data = "<req>")]
pub async fn add_slug_page_endpoint(
    req: Json<SlugRequest>,
    cfg: &State<Config>,
    cloudflare: &State<Cloudflare>,
) -> Result<Json<JsonValue>, Status> {
    updater::add_slug_page(
        &req.user_id,
        &req.business_id,
        &req.slug,
        &req.site_id,
        req.rewrite_target.as_deref(),
    )
    .map_err(|_| Status::InternalServerError)?;
    Ok(Json(json!({
        "status": 200,
        "message": "Slug page added successfully"
    })))
}

#[put("/slug", data = "<req>")]
pub async fn update_slug_page_endpoint(
    req: Json<SlugRequest>,
    cfg: &State<Config>,
) -> Result<Json<JsonValue>, Status> {
    updater::update_slug_page(
        &req.user_id,
        &req.business_id,
        &req.slug,
        &req.site_id,
        req.rewrite_target.as_deref(),
    )
    .map_err(|_| Status::InternalServerError)?;
    Ok(Json(json!({
        "status": 200,
        "message": "Slug page updated successfully"
    })))
}

#[delete("/slug", data = "<req>")]
pub async fn delete_slug_page_endpoint(
    req: Json<SlugRequest>,
    cfg: &State<Config>,
    cloudflare: &State<Cloudflare>,
) -> Result<Json<JsonValue>, Status> {
    updater::delete_slug_page(&req.user_id, &req.business_id, &req.slug)
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(json!({
        "status": 200,
        "message": "Slug page deleted successfully"
    })))
}

#[options("/<_..>")]
fn handle_cors() -> Status {
    Status::Ok
}

pub async fn build_endpoints() -> Rocket<Build> {
    rocket::build().mount(
        "/api",
        routes![
            handle_cors,
            register,
            auth,
            whoami,
            create_domain_endpoint,
            delete_domain_endpoint,
            add_slug_page_endpoint,
            update_slug_page_endpoint,
            delete_slug_page_endpoint,
            verify_account,
        ],
    )
}
