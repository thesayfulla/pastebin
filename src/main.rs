use crate::errors::not_found;
use crate::routes::configure_routes;
use actix_web::http::StatusCode;
use actix_web::middleware::{ErrorHandlers, Logger};
use actix_web::{web, App, HttpServer};
use minijinja::path_loader;
use minijinja_autoreload::AutoReloader;
use std::env;
use std::path::PathBuf;

mod errors;
mod handlers;
mod renderer;
mod routes;
mod db;

use db::Database;

pub struct AppState {
    pub db: Database,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let enable_template_autoreload = env::var("TEMPLATE_AUTORELOAD").as_deref() == Ok("true");

    if enable_template_autoreload {
        log::info!("template auto-reloading is enabled");
    } else {
        log::info!(
            "template auto-reloading is disabled; run with TEMPLATE_AUTORELOAD=true to enable"
        );
    }

    let tmpl_reloader = AutoReloader::new(move |notifier| {
        let mut env: minijinja::Environment<'static> = minijinja::Environment::new();

        let tmpl_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");

        if enable_template_autoreload {
            notifier.watch_path(&tmpl_path, true);
        }

        env.set_loader(path_loader(tmpl_path));

        Ok(env)
    });

    let tmpl_reloader = web::Data::new(tmpl_reloader);

    // Initialize database using the new Database struct
    let db = Database::new("pastes.db").expect("Failed to initialize database");
    let app_state = web::Data::new(AppState { db });

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .configure(configure_routes)
            .app_data(tmpl_reloader.clone())
            .app_data(app_state.clone())
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found))
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}