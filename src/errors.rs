use std::fmt;

pub type Result<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug)]
pub enum JWTCError {
    TokenExpired,
}

#[derive(Debug)]
pub enum ErrorKind {
    IOError(std::io::Error),
    JWTError(jsonwebtoken::errors::Error),
    JWTCreationError(JWTCError),
    InvalidValue,
}

impl From<std::io::Error> for ErrorKind {
    fn from(error: std::io::Error) -> Self {
        ErrorKind::IOError(error)
    }
}

impl From<jsonwebtoken::errors::Error> for ErrorKind {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        ErrorKind::JWTError(error)
    }
}

impl From<JWTCError> for ErrorKind {
    fn from(error: JWTCError) -> Self {
        ErrorKind::JWTCreationError(error)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ErrorKind::IOError(err) => err.to_string(),
            ErrorKind::JWTError(err) => err.to_string(),
            ErrorKind::JWTCreationError(err) => match err {
                JWTCError::TokenExpired => "Token expired".to_string(),
            },
            ErrorKind::InvalidValue => "Invalid value".to_string(),
        };

        write!(f, "{}", msg)
    }
}