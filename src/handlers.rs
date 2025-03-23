use crate::renderer::*;
use crate::AppState;
use actix_web::http::header;
use actix_web::{web, HttpResponse, Responder};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

#[derive(serde::Deserialize)]
pub struct FormData {
    pub title: String,
    pub content: String,
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

    match data.db.insert_paste(&token, &content.title, &content.content) {
        Ok(_) => HttpResponse::SeeOther()
            .insert_header((header::LOCATION, format!("/share/{}", token)))
            .finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save paste"),
    }
}

pub async fn share(
    token: web::Path<String>,
    tmpl_env: MiniJinjaRenderer,
    data: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let paste = data.db.get_paste_by_token(&token);

    match paste {
        Ok((content, title)) => {
            tmpl_env.render(
                "paste.html",
                minijinja::context! {
                    content => content,
                    title => title,
                    token => token.to_string(),
                },
            )
        },
        Err(_) => {
            tmpl_env.render("error.html", minijinja::context! {
                status_code => "404",
                error => "Not found",
            })
        }
    }    
}

pub async fn view_raw(token: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let content = data.db.get_content_by_token(&token);

    match content {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(_) => HttpResponse::NotFound().body("404 not found"),
    }
}