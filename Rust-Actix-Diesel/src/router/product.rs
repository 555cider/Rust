use actix_web::web;

use crate::{db_connection, handler};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.app_data(web::Data::new(db_connection::establish_connection()))
        .service(
            web::resource("")
                .route(web::get().to(handler::product::list_product))
                .route(web::post().to(handler::product::create_product)),
        )
        .service(
            web::resource("/{id}")
                .route(web::get().to(handler::product::find_product))
                .route(web::delete().to(handler::product::delete_product))
                .route(web::patch().to(handler::product::update_product)),
        );
}
