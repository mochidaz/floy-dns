use bcrypt::verify;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json::json;
use rocket_dyn_templates::handlebars::JsonValue;

use crate::jwt::generate_token;
use crate::models::User;
use crate::utils::{hash_password, validate_username};
use crate::writers::{Writer, WriterConn};

#[post("/register", data = "<data>")]
pub async fn register(
    data: Json<User>,
    writer: WriterConn,
) -> Result<Json<JsonValue>, Status> {
    let user = data.into_inner();

    if writer.exists(&user.username).await.map_err(|_| Status::InternalServerError)? {
        return Err(Status::Conflict);
    }

    let validate = validate_username(&user.username);

    if !validate {
        return Err(Status::BadRequest);
    }

    let hash = hash_password(&user.password);

    let message = format!("{}:{}\r", user.username, hash);

    writer.write(&message).await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(json!(
        {
            "status": 200,
            "message": "User registered successfully"
        }
    )))
}

#[post("/login", data = "<data>")]
pub async fn login(
    data: Json<User>,
    writer: WriterConn,
) -> Result<Json<JsonValue>, Status> {
    let user = data.into_inner();

    let search = match writer.find(&user.username).await.map_err(|_| Status::InternalServerError)? {
        Some(search) => search,
        None => return Err(Status::NotFound),
    };

    let password = match search.get(&user.username) {
        Some(password) => password,
        None => return Err(Status::NotFound),
    };

    if !verify(&user.password, &password).map_err(|_| Status::InternalServerError)? {
        return Err(Status::Unauthorized);
    }

    let token = generate_token(&user.username).map_err(|_| Status::InternalServerError)?;

    Ok(Json(json!(
        {
            "status": 200,
            "message": "User logged in successfully",
            "token": token,
        }
    )))
}
