use crate::auth::{hash_password, verify_password, DEFAULT_SCOPE};
use crate::errors::UserError;
use entity::user;
use entity::user::Entity as User;
use log::{error, info};
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*};

/**
 * Logs the user in and creates a session for them.
 */
pub async fn login_user(
    db: &DatabaseConnection,
    username: String,
    password: String,
) -> Result<i64, UserError> {
    let user = match User::find()
        .filter(user::Column::Username.eq(username.as_str()))
        .one(db)
        .await
    {
        Ok(user_opt) => match user_opt {
            Some(user) => user,
            None => return Err(UserError::UnknownLogin),
        },
        Err(e) => {
            error!("Failed to find user '{}' by id: {}", username, e);
            return Err(UserError::Internal);
        }
    };

    match verify_password(user.password_hash, password) {
        Ok(result) => {
            if result {
                Ok(user.id)
            } else {
                info!("Failed login attempt for user {}", username);
                Err(UserError::InvalidLogin)
            }
        }
        Err(e) => {
            error!("Failed to verify password: {}", e);

            Err(UserError::InvalidLogin)
        }
    }
}

/**
 * Checks if a user exists by username.
 */
async fn check_users_exists(db: &DatabaseConnection, username: String) -> Result<bool, UserError> {
    match User::find()
        .filter(user::Column::Username.eq(username.as_str()))
        .one(db)
        .await
    {
        Ok(user_opt) => Ok(user_opt.is_some()),
        Err(e) => {
            error!("Failed to find user by id: {}", e);
            Err(UserError::Internal)
        }
    }
}

/**
 * Adds a new user to the db.
 */
pub async fn add_user(
    db: &DatabaseConnection,
    username: String,
    password: String,
) -> Result<i64, UserError> {
    let user_exists = check_users_exists(db, username.clone()).await.unwrap();

    if user_exists {
        return Err(UserError::UserExists);
    }
    // {} curly brackets are here because I want to visually scope the password hash
    {
        let pwh = hash_password(password.to_owned())
            .map_err(|_| UserError::Internal)
            .unwrap();

        let user_model = user::ActiveModel {
            username: Set(username.to_owned()),
            password_hash: Set(pwh.to_owned()),
            scope: Set(DEFAULT_SCOPE.to_owned()),
            created_at: Set(chrono::Local::now().naive_local()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| {
            error!(
                "Failed to insert new user '{}' to database: {}",
                username, e
            );
            UserError::Internal
        })
        .unwrap();

        Ok(user_model.id)
    }
}
