use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};

pub const DEFAULT_SCOPE: &str = "insert,delete,edit";

pub(crate) fn hash_password(password: String) -> Result<String, argon2::password_hash::Error> {
    let password = password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    Ok(argon2.hash_password(password, &salt).unwrap().to_string())
}

pub(crate) fn verify_password(
    pwh: String,
    password: String,
) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(&pwh).unwrap();

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub(crate) fn check_password(password: String) -> bool {
    if password.is_empty() {
        return false;
    }

    true
}
