use actix_files as fs;
use actix_utils::future::{ready, Ready};
use actix_web::{
    dev::{self, ServiceResponse},
    error,
    http::{header, StatusCode},
    middleware::{ErrorHandlerResponse, ErrorHandlers, Logger},
    web, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use actix_web_lab::respond::Html;
use minijinja::path_loader;
use minijinja_autoreload::AutoReloader;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use rusqlite::{params, Connection};
use std::sync::Mutex;
use std::{env, path::PathBuf};

struct AppState {
    db: Mutex<Connection>,
}
#[derive(serde::Deserialize)]
struct FormData {
    title: String,
    content: String,
}

struct MiniJinjaRenderer {
    tmpl_env: web::Data<AutoReloader>,
}

impl MiniJinjaRenderer {
    fn render(
        &self,
        tmpl: &str,
        ctx: impl Into<minijinja::value::Value>,
    ) -> actix_web::Result<Html> {
        self.tmpl_env
            .acquire_env()
            .map_err(|_| error::ErrorInternalServerError("could not acquire template env"))?
            .get_template(tmpl)
            .map_err(|_| error::ErrorInternalServerError("could not find template"))?
            .render(ctx.into())
            .map(Html)
            .map_err(|err| {
                log::error!("{err}");
                error::ErrorInternalServerError("template error")
            })
    }
}

impl FromRequest for MiniJinjaRenderer {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _pl: &mut dev::Payload) -> Self::Future {
        let tmpl_env = <web::Data<minijinja_autoreload::AutoReloader>>::extract(req)
            .into_inner()
            .unwrap();

        ready(Ok(Self { tmpl_env }))
    }
}

async fn index(tmpl_env: MiniJinjaRenderer) -> actix_web::Result<impl Responder> {
    tmpl_env.render("index.html", ())
}

async fn submit(content: web::Form<FormData>, data: web::Data<AppState>) -> impl Responder {
    let token: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    let conn = data.db.lock().unwrap();
    conn.execute(
        "INSERT INTO pastes (token, title, content) VALUES (?, ?, ?)",
        params![&token, &content.title, &content.content],
    )
    .expect("Failed to insert into db");

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, format!("/share/{}", token)))
        .finish()
}

async fn paste(
    token: web::Path<String>,
    tmpl_env: MiniJinjaRenderer,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let conn = data.db.lock().unwrap();

    // Execute the SQL query to retrieve both content and title
    let paste = conn
        .query_row(
            "SELECT content, title FROM pastes WHERE token = ?",
            params![token.to_string()],
            |row| {
                let content: String = row.get(0)?;
                let title: String = row.get(1)?;
                Ok((content, title))
            },
        ).unwrap(); // Handle this more gracefully in production

    // Extract content and title from the result tuple
    let (content, title) = paste;

    // Render the template with content and title in the context
    tmpl_env.render(
        "paste.html",
        minijinja::context! {
            content => content.to_string(),
            title => title.to_string(),
        },
    )
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // If TEMPLATE_AUTORELOAD is set, then the path tracking is enabled.
    let enable_template_autoreload = env::var("TEMPLATE_AUTORELOAD").as_deref() == Ok("true");

    if enable_template_autoreload {
        log::info!("template auto-reloading is enabled");
    } else {
        log::info!(
            "template auto-reloading is disabled; run with TEMPLATE_AUTORELOAD=true to enable"
        );
    }

    // The closure is invoked every time the environment is outdated to recreate it.
    let tmpl_reloader = AutoReloader::new(move |notifier| {
        let mut env: minijinja::Environment<'static> = minijinja::Environment::new();

        let tmpl_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");

        // if watch_path is never called, no fs watcher is created
        if enable_template_autoreload {
            notifier.watch_path(&tmpl_path, true);
        }

        env.set_loader(path_loader(tmpl_path));

        Ok(env)
    });

    let tmpl_reloader = web::Data::new(tmpl_reloader);

    let db = Connection::open("pastes.db").expect("Failed to open db");
    db.execute(
        "CREATE TABLE IF NOT EXISTS pastes (token TEXT PRIMARY KEY, title VARCHAR, content TEXT)",
        params![],
    )
    .expect("Failed to create table");

    let app_state = web::Data::new(AppState { db: Mutex::new(db) });

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(tmpl_reloader.clone())
            .app_data(app_state.clone())
            .service(
                fs::Files::new("/static", "./static")
                    .show_files_listing()
                    .use_last_modified(true),
            )
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/submit").route(web::post().to(submit)))
            .service(web::resource("/share/{token}").route(web::get().to(paste)))
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found))
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

/// Error handler for a 404 Page not found error.
fn not_found<B>(svc_res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let res = get_error_response(&svc_res, "Page not found");

    Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
        svc_res.into_parts().0,
        res.map_into_right_body(),
    )))
}

/// Generic error handler.
fn get_error_response<B>(res: &ServiceResponse<B>, error: &str) -> HttpResponse {
    let req = res.request();

    let tmpl_env = MiniJinjaRenderer::extract(req).into_inner().unwrap();

    // Provide a fallback to a simple plain text response in case an error occurs during the
    // rendering of the error page.
    let fallback = |err: &str| {
        HttpResponse::build(res.status())
            .content_type(header::ContentType::plaintext())
            .body(err.to_string())
    };

    let ctx = minijinja::context! {
        error => error,
        status_code => res.status().as_str(),
    };

    match tmpl_env.render("error.html", ctx) {
        Ok(body) => body
            .customize()
            .with_status(res.status())
            .respond_to(req)
            .map_into_boxed_body(),

        Err(_) => fallback(error),
    }
}
