use std::collections::HashMap;
use std::io::{Write, Read};
use std::fs;
use std::convert;
use std::path::{Path, PathBuf};
use std::convert::AsRef;
use std::ops::Deref;

use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::serde_json;

use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;

use crate::errors::ErrorKind;


pub struct Writer<T: AsRef<Path>> {
    text_file: T,
    file: Mutex<BufWriter<File>>,
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
            file: Mutex::new(BufWriter::new(file)),
        })
    }

    pub async fn write(&self, text: &str) -> Result<(), std::io::Error> {
        let mut file = self.file.lock().await;
        file.write_all(text.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    pub async fn flush(&self) -> Result<(), std::io::Error> {
        let mut file = self.file.lock().await;
        file.flush().await?;
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool, std::io::Error> {
        let mut file = self.file.lock().await;
        let mut contents = String::new();

        file.read_to_string(&mut contents).await?;

        for line in contents.split("\r") {
            let map: Vec<String> = line.trim()
                .split(":")
                .map(|s| s.to_string())
                .collect();
            if map[0] == key {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn find(&self, key: &str) -> Result<Option<HashMap<String, String>>, std::io::Error> {
        let mut file = self.file.lock().await;
        let mut contents = String::new();

        file.read_to_string(&mut contents).await?;

        for line in contents.split("\r") {
            let map: Vec<String> = line.trim()
                .split(":")
                .map(|s| s.to_string())
                .collect();
            if map[0] == key {
                return Ok(Some(HashMap::from([
                    (map[0].clone(), map[1].clone())
                ])))
            }
        }
        Ok(None)
    }
}

pub struct WriterConn(pub Writer<String>);

#[rocket::async_trait]
impl <'r> FromRequest<'r> for WriterConn {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let text_file = request
            .guard::<&rocket::State<Writer<String>>>()
            .await
            .succeeded()
            .unwrap()
            .text_file
            .clone();

        match Writer::new(text_file.clone()).await {
            Ok(writer) => Outcome::Success(WriterConn(writer)),
            Err(_) => Outcome::Failure((rocket::http::Status::InternalServerError, ()))
        }
    }
}

impl Deref for WriterConn {
    type Target = Writer<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}