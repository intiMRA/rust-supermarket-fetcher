use actix_web::web;

use super::handlers;

/// Configure all API routes.
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/shopping-list", web::post().to(handlers::shopping_list))
            .route("/shopping-list-by-ids", web::post().to(handlers::shopping_list_by_ids))
            .route("/paginated-list", web::post().to(handlers::paginated_list))
            .route("/health", web::get().to(handlers::health)),
    );
}
