use rocket::serde::{Serialize, Deserialize};

use std::fmt;

#[derive(Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct DNS {
    pub record_type: RecordType,
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub enum RecordType {
    A,
    AAAA,
    CNAME,
}

impl fmt::Display for RecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::A => "A",
            Self::AAAA => "AAAA",
            Self::CNAME => "CNAME",
        };

        write!(f, "{}", msg)
    }
}