use actix_cors::Cors;
use actix_web::{http::header, middleware, App, HttpServer};
use syntropic_api::{configure, data};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = data().await;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .configure(configure)
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["POST", "GET"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
                    .supports_credentials()
                    .max_age(3600),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
    });

    server.bind("localhost:8080").unwrap().run().await
}
