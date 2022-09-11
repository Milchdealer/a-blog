use actix_files::Files as Fs;
use actix_identity::IdentityMiddleware;
use actix_session::SessionMiddleware;
use actix_web::{cookie::Key, middleware, web, App, HttpServer};
use log::info;
use migration::{Migrator, MigratorTrait};
use sea_orm::DatabaseConnection;
use session_store::SqliteSessionStore;
use std::env;
use tera::Tera;

mod api;
mod auth;
mod db;
mod errors;
mod session_store;

#[derive(Debug, Clone)]
pub struct AppState {
    templates: tera::Tera,
    conn: DatabaseConnection,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let mut template_dir = env::var("TERA_TEMPLATE_DIR").expect("TERA_TEMPLATE_DIR is not set");
    let secret_key = env::var("SESSION_SECRET_KEY").expect("SESSION_SECRET_KEY is not set");
    let secret_key = Key::from(secret_key.as_bytes());
    let host = env::var("HOST").unwrap_or("127.0.0.1".into());
    let port = env::var("PORT").unwrap_or("8080".into());
    let server_url = format!("{}:{}", host, port);

    let conn = sea_orm::Database::connect(&db_url).await.unwrap();
    Migrator::up(&conn, None).await.unwrap();
    if !template_dir.ends_with("/**/*") {
        template_dir.push_str("/templates/**/*");
    }
    let sqlite_session_store = SqliteSessionStore::new(db_url).await;
    let templates = Tera::new(template_dir.as_str()).unwrap();
    let state = AppState { templates, conn };

    let server = HttpServer::new(move || {
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(SessionMiddleware::new(
                sqlite_session_store.clone(),
                secret_key.clone(),
            ))
            .service(Fs::new("/static", "./static"))
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Logger::default())
            .configure(init)
    })
    .bind(&server_url)?;

    info!("Starting server at {}", server_url);
    server.run().await
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(api::index);
    cfg.service(api::submit);
    cfg.service(api::login);
    cfg.service(api::logout);
    cfg.service(api::register);
}
