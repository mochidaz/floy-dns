#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused)]

#[macro_use]
extern crate rocket;

use crate::cloudflare::cloudflare::Cloudflare;
use crate::common::errors::build_catchers;
use crate::common::writers::Writer;
use crate::config::Config;
use crate::endpoints::build_endpoints;

mod cloudflare;
mod common;
mod config;
mod endpoints;
mod models;
mod parser;
mod updater;

#[launch]
async fn rocket() -> rocket::Rocket<rocket::Build> {
    dotenv::dotenv().ok();

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
