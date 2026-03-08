use actix_web::web;

use super::handlers;

/// Configure all API routes.
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/shopping-list", web::post().to(handlers::shopping_list))
            .route("/health", web::get().to(handlers::health)),
    );
}
