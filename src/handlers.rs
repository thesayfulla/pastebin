use crate::renderer::*;
use crate::AppState;
use actix_web::http::header;
use actix_web::{web, HttpResponse, Responder};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusqlite::params;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    content: String,
}

pub async fn index(tmpl_env: MiniJinjaRenderer) -> actix_web::Result<impl Responder> {
    tmpl_env.render("index.html", ())
}

pub async fn submit(content: web::Form<FormData>, data: web::Data<AppState>) -> impl Responder {
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

pub async fn share(
    token: web::Path<String>,
    tmpl_env: MiniJinjaRenderer,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let conn = data.db.lock().unwrap();

    let paste = conn
        .query_row(
            "SELECT content, title FROM pastes WHERE token = ?",
            params![token.to_string()],
            |row| {
                let content: String = row.get(0)?;
                let title: String = row.get(1)?;
                Ok((content, title))
            },
        )
        .unwrap();

    let (content, title) = paste;

    tmpl_env.render(
        "paste.html",
        minijinja::context! {
            content => content.to_string(),
            title => title.to_string(),
            token => token.to_string(),
        },
    )
}

pub async fn view_raw(token: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let content = conn
        .query_row(
            "SELECT content FROM pastes WHERE token = ?",
            params![token.to_string()],
            |row| row.get::<_, String>(0),
        )
        .unwrap();

    HttpResponse::Ok().body(format!("{}", content))
}
