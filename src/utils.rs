use bcrypt::{hash, verify, DEFAULT_COST};

pub fn validate_username(s: &String) -> bool {
    s.chars().all(|c| c.is_alphanumeric())
}

pub fn hash_password(s: &String) -> String {
    hash(s, DEFAULT_COST).unwrap()
}

#[cfg(test)]
mod tests {
    use bcrypt::verify;
    use super::*;

    #[test]
    fn test_validate_username() {
        let username_success = "username123".to_string();
        let username_fail = "username 123".to_string();

        assert_eq!(true, validate_username(&username_success));
        assert_eq!(false, validate_username(&username_fail));
    }

    #[test]
    fn test_hash_password() {
        let password = "password".to_string();
        let hashed_password = hash_password(&password);

        assert_ne!(password, hashed_password);
        assert_eq!(true, verify(&password, &hashed_password).unwrap());
    }
}