use crate::handlers::*;
use actix_files as fs;
use actix_web::web;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        fs::Files::new("/static", "./static")
            .show_files_listing()
            .use_last_modified(true),
    )
    .service(web::resource("/").route(web::get().to(index)))
    .service(web::resource("/submit").route(web::post().to(submit)))
    .service(web::resource("/share/{token}").route(web::get().to(share)))
    .service(web::resource("/share/{token}/raw").route(web::get().to(view_raw)));
}
