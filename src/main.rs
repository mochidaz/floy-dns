#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused)]

#[macro_use]
extern crate rocket;

use rocket::fairing::Fairing;
use rocket::http::{Method, Status};

use crate::cloudflare::Cloudflare;
use crate::config::Config;
use crate::endpoints::build_endpoints;
use crate::errors::build_catchers;
use crate::writers::Writer;

mod cloudflare;
mod config;
mod endpoints;
mod errors;
mod jwt;
mod models;
mod utils;
mod writers;

#[launch]
async fn rocket() -> rocket::Rocket<rocket::Build> {
    let config = Config::new();
    let writer = Writer::new(config.database_path.clone()).await.unwrap();
    let cloudflare = Cloudflare::new(config.clone()).await;

    build_endpoints()
        .await
        .manage(writer)
        .manage(config)
        .manage(cloudflare)
        .attach(build_catchers().await)
}
