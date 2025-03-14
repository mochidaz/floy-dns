use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;
use bcrypt::verify;
use reqwest::{get, redirect};
use rocket::{Build, Rocket, State};
use rocket::http::Status;
use rocket::log::private::{log, logger, Level, Log};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json::json;
use rocket_dyn_templates::handlebars::JsonValue;

use crate::cloudflare::Cloudflare;
use crate::config::Config;
use crate::errors::ErrorKind;
use crate::errors::ErrorKind::Error;
use crate::jwt::{ApiKey, generate_token, read_token};
use crate::models::{DNS, Login, User, WhoAmI};
use crate::utils::{
    find_subdomain_claim, hash_password, send_verification_email, validate_email, validate_username,
};
use crate::writers::Writer;

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

#[get("/dns")]
async fn dns(
    config: &State<Config>,
    cloudflare: &State<Cloudflare>,
    writer: &State<Writer<String>>,
    key: ApiKey,
) -> Result<Json<JsonValue>, Status> {
    let user = match find_subdomain_claim(&writer, &key.0)
        .await
        .map_err(|_| Status::InternalServerError)
    {
        Ok(subdomain_claim) => match subdomain_claim {
            Some(subdomain_claim) => subdomain_claim,
            None => return Err(Status::NotFound),
        },
        Err(_) => return Err(Status::InternalServerError),
    };

    let dns_entry = match cloudflare
        .get_subdomain_dns_record(&user.subdomain_claim, false)
        .await
        .map_err(|_| Status::InternalServerError)
    {
        Ok(dns_entry) => dns_entry,
        Err(_) => return Err(Status::NotFound),
    }
    .0;

    Ok(Json(json!(
        {
            "status": 200,
            "message": "DNS record found",
            "ip": dns_entry,
        }
    )))
}

#[post("/dns", data = "<dns>")]
async fn dns_add(
    config: &State<Config>,
    cloudflare: &State<Cloudflare>,
    writer: &State<Writer<String>>,
    dns: Json<DNS>,
) -> Result<Json<JsonValue>, Status> {
    match cloudflare
        .add_subdomain_dns_record(&dns.subdomain, &dns.ip)
        .await
        .map_err(|e| {
            Status::InternalServerError
        }) {
        Ok(_) => {}
        Err(e) => {
            return Err(Status::NotFound);
        },
    };

    let sites_available_dir = "/etc/nginx/sites-available/";
    let sites_enabled_dir = "/etc/nginx/sites-enabled/";
    let file_name = format!("{}", dns.subdomain);
    let available_path = Path::new(sites_available_dir).join(&file_name);
    let enabled_path = Path::new(sites_enabled_dir).join(&file_name);

    let nginx_template = format!(
        r#"server {{
    listen 80;
    server_name {subdomain}.cciunitel.com;

    root {web_path};
    index index.html;

    location / {{
        try_files $uri $uri/ =404;
    }}
}}"#,
        subdomain = dns.subdomain,
        web_path = dns.web_path
    );

    if let Err(e) = fs::write(&available_path, nginx_template) {
        return Err(Status::InternalServerError);
    }

    if !enabled_path.exists() {
        if let Err(e) = symlink(&available_path, &enabled_path) {
            return Err(Status::InternalServerError);
        }
    }

    if let Err(e) = Command::new("nginx")
        .arg("-s")
        .arg("reload")
        .output() {
        return Err(Status::InternalServerError);
    }

    Ok(Json(json!(
        {
            "status": 200,
            "message": "DNS record added and nginx config created",
            "ip": &dns.ip,
            "subdomain": &dns.subdomain,
            "web_path": &dns.web_path
        }
    )))
}

#[put("/dns", data = "<dns>")]
async fn dns_update(
    config: &State<Config>,
    cloudflare: &State<Cloudflare>,
    writer: &State<Writer<String>>,
    dns: Json<DNS>,
) -> Result<Json<JsonValue>, Status> {
    let user = match find_subdomain_claim(&writer, &dns.subdomain)
        .await
        .map_err(|_| Status::InternalServerError)
    {
        Ok(subdomain_claim) => match subdomain_claim {
            Some(subdomain_claim) => subdomain_claim,
            None => return Err(Status::NotFound),
        },
        Err(_) => return Err(Status::InternalServerError),
    };

    match cloudflare
        .update_subdomain_dns_record(&user.subdomain_claim, &dns.ip)
        .await
        .map_err(|_| Status::InternalServerError)
    {
        Ok(_) => {}
        Err(_) => return Err(Status::NotFound),
    };

    Ok(Json(json!(
        {
            "status": 200,
            "message": "DNS record updated",
            "ip": &dns.ip,
        }
    )))
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
            dns,
            dns_add,
            dns_update,
            verify_account,
        ],
    )
}
