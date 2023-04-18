use std::collections::HashMap;
use std::convert;
use std::convert::AsRef;
use std::fs;
use std::io::SeekFrom;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::serde_json;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::Mutex;

use crate::errors::ErrorKind;
use crate::models::User;

pub struct Writer<T: AsRef<Path>> {
    text_file: T,
    file: Mutex<File>,
}

impl<T: AsRef<Path>> Writer<T> {
    pub async fn new(text_file: T) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .read(true)
            .open(&text_file)
            .await?;
        Ok(Self {
            text_file,
            file: Mutex::new(file),
        })
    }

    pub async fn write(&self, text: &str) -> Result<(), std::io::Error> {
        let mut file = self.file.lock().await;
        file.write(text.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    pub async fn flush(&self) -> Result<(), std::io::Error> {
        let mut file = self.file.lock().await;
        file.flush().await?;
        Ok(())
    }

    pub async fn find(&self, key: &str) -> Result<Option<User>, std::io::Error> {
        let mut file = self.file.lock().await;

        file.seek(SeekFrom::Start(0)).await?;

        let mut content = String::new();

        file.read_to_string(&mut content).await?;

        for i in content.split("\r").collect::<Vec<&str>>() {
            let map: Vec<String> = i.trim().split(":").map(|s| s.to_string()).collect();
            if map[0].is_empty() {
                continue;
            }
            if map[0] == key || map[1] == key {
                return Ok(Some(User {
                    subdomain_claim: map[0].clone(),
                    email: map[1].clone(),
                    password: map[2].clone(),
                }));
            }
        }

        Ok(None)
    }
}

pub struct WriterConn(pub Writer<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for WriterConn {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let text_file = request
            .guard::<&rocket::State<Writer<String>>>()
            .await
            .succeeded()
            .unwrap()
            .text_file
            .clone();

        match Writer::new(text_file.clone()).await {
            Ok(writer) => Outcome::Success(WriterConn(writer)),
            Err(_) => Outcome::Failure((rocket::http::Status::InternalServerError, ())),
        }
    }
}

impl Deref for WriterConn {
    type Target = Writer<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
