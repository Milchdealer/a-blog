use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub enum UserError {
    #[display(fmt = "Missing data in field: {}", field)]
    MissingData { field: String },
    #[display(fmt = "Something went front")]
    Internal,
    #[display(fmt = "User already exists")]
    UserExists,
    #[display(fmt = "Invalid login credentials")]
    InvalidLogin,
    #[display(fmt = "Unknown login")]
    UnknownLogin,
}
impl error::ResponseError for UserError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }
    fn status_code(&self) -> StatusCode {
        match *self {
            UserError::MissingData { .. } => StatusCode::BAD_REQUEST,
            UserError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            UserError::UserExists { .. } => StatusCode::CONFLICT,
            UserError::InvalidLogin { .. } => StatusCode::UNAUTHORIZED,
            UserError::UnknownLogin { .. } => StatusCode::BAD_REQUEST,
        }
    }
}
