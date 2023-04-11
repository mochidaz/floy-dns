#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused)]

#[macro_use]
extern crate rocket;

use rocket::http::{Method, Status};
use rocket::fairing::Fairing;

use crate::writers::Writer;

mod writers;
mod errors;
mod utils;
mod endpoints;
mod models;
mod jwt;

#[launch]
async fn rocket() -> rocket::Rocket<rocket::Build> {
    let writer = Writer::new(String::from("test.txt")).await.unwrap();
    rocket::build()
        .manage(writer)
        .mount("/", routes![endpoints::register, endpoints::login])
}