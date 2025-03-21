use std::convert::AsRef;
use std::io::SeekFrom;
use std::path::Path;

use rocket::request::FromRequest;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;

use crate::models::User;

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
