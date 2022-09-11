use crate::auth::check_password;
use crate::db::{add_user, login_user};
use crate::errors::UserError;
use actix_http::HttpMessage;
use actix_identity::Identity;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use chrono::{self, NaiveDate};
use entity::entry::Entity as Entry;
use entity::{entry, user};
use log::error;
use sea_orm::EntityTrait;
use sea_orm::{entity::*, query::*};
use serde::Deserialize;

const MAX_ENTRIES_IN_LIST: u64 = 5;

#[get("/")]
pub async fn index(
    data: web::Data<crate::AppState>,
    user: Option<Identity>,
) -> Result<HttpResponse, UserError> {
    let template = &data.templates;
    let mut ctx = tera::Context::new();

    if let Some(user) = user {
        let conn = &data.conn;
        let today = chrono::offset::Utc::now().date_naive();
        let user_id = match user.id() {
            Ok(username) => username.parse::<i64>().unwrap(),
            Err(e) => {
                error!("Failed to read username from user Identity: {}", e);
                return Err(UserError::Internal);
            }
        };

        let results = Entry::find()
            .join(JoinType::InnerJoin, entry::Relation::User.def())
            .filter(Condition::all().add(user::Column::Id.eq(user_id)))
            .order_by_desc(entry::Column::Id)
            .limit(MAX_ENTRIES_IN_LIST)
            .all(conn)
            .await
            .unwrap_or_default();

        ctx.insert("is_logged_in", "true");
        ctx.insert("today", &today.to_string());
        ctx.insert("workouts", &results);

        let body = template
            .render("index.html.tera", &ctx)
            .map_err(|_| UserError::Internal);
        match body {
            Ok(body) => Ok(HttpResponse::Ok().content_type("text/html").body(body)),
            Err(e) => Err(e),
        }
    } else {
        let body = template
            .render("login.html.tera", &ctx)
            .map_err(|_| UserError::Internal);
        match body {
            Ok(body) => Ok(HttpResponse::Ok().content_type("text/html").body(body)),
            Err(e) => Err(e),
        }
    }
}

#[derive(Deserialize)]
pub struct FormData {
    entry_date: String,
    exercise: String,
    sets: Option<u32>,
    reps_duration: u32,
    load: Option<String>,
}

#[post("/api/submit")]
pub async fn submit(
    post_form: web::Form<FormData>,
    data: web::Data<crate::AppState>,
    user: Identity,
) -> impl Responder {
    let conn = &data.conn;
    let form = post_form.into_inner();

    let date = NaiveDate::parse_from_str(&form.entry_date, "%Y-%m-%d").unwrap_or_default();
    let sets = match form.sets {
        Some(sets) => sets,
        None => 1,
    };

    match user.id() {
        Ok(user_id) => {
            entry::ActiveModel {
                user_id: Set(user_id.parse::<i64>().unwrap()),
                entry_date: Set(date),
                exercise: Set(form.exercise.to_owned()),
                sets: Set(sets),
                reps_duration: Set(form.reps_duration.to_owned()),
                load: Set(form.load.to_owned()),
                ..Default::default()
            }
            .save(conn)
            .await
            .expect("Could not insert workout");

            Ok(HttpResponse::Created()
                .append_header(("HX-Refresh", "true"))
                .finish())
        }
        Err(e) => {
            error!(
                "Failed to add new entry to db because no user found in Identity: {}",
                e
            );
            Err(UserError::Internal)
        }
    }
}

#[derive(Deserialize)]
pub struct InputUser {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RedirectInfo {
    redirect_url: Option<String>,
}

#[post("/api/login")]
pub async fn login(
    post_form: web::Form<InputUser>,
    data: web::Data<crate::AppState>,
    redirect_url: web::Query<RedirectInfo>,
    request: HttpRequest,
) -> impl Responder {
    let conn = &data.conn;
    let form = post_form.into_inner();
    let redirect_url = redirect_url.0.redirect_url.unwrap_or("/".into());

    match login_user(conn, form.username.clone(), form.password).await {
        Ok(user_id) => {
            Identity::login(&request.extensions(), user_id.to_string()).unwrap();

            return Ok(HttpResponse::Ok()
                .append_header(("HX-Redirect", redirect_url))
                .finish());
        }
        Err(e) => Err(e),
    }
}

#[post("/api/logout")]
pub async fn logout(user: Identity) -> impl Responder {
    user.logout();
    HttpResponse::Ok()
        .append_header(("HX-Refresh", "true"))
        .finish()
}

#[post("/api/register")]
pub async fn register(
    post_form: web::Form<InputUser>,
    data: web::Data<crate::AppState>,
    request: HttpRequest,
) -> Result<HttpResponse, UserError> {
    let conn = &data.conn;
    let form = post_form.into_inner();

    // Basic sanity checks
    if form.username.is_empty() || !check_password(form.password.clone()) {
        return Err(UserError::InvalidLogin);
    }

    let result: Result<i64, UserError> = add_user(conn, form.username, form.password).await;

    match result {
        Ok(user_id) => {
            Identity::login(&request.extensions(), user_id.to_string()).unwrap();

            Ok(HttpResponse::Created()
                .append_header(("HX-Location", "/"))
                .finish())
        }
        Err(e) => Err(e),
    }
}
