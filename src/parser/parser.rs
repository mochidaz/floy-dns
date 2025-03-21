use pest::Parser;
use pest_derive::Parser;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;

#[derive(Parser)]
#[grammar = "nginx.pest"]
pub struct NginxParser;

pub fn validate_config(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;
    NginxParser::parse(Rule::server, &content)
        .map(|_| ())
        .map_err(|err| {
            eprintln!("Error parsing {}: {}", path.display(), err);
            Error::new(ErrorKind::Other, "Invalid nginx config")
        })
}
