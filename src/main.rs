extern crate dotenv;
#[macro_use]
extern crate log;

extern crate actix_web;

use actix_web::{get, App, HttpRequest, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn hello() -> impl Responder {
	debug!("Called hello.");
	HttpResponse::Ok().body("hello, world")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	dotenv::dotenv().ok();
	env_logger::builder().format_timestamp_millis().init();
	let user = std::env::var("CONFIGURED_USER").unwrap_or("unknown user".to_string());
	info!("Hello, {}!", user);

	let bindaddr = std::env::var("TFS_BIND_ADDR").unwrap_or("127.0.0.1:8080".to_string());
	HttpServer::new(|| {
		App::new()
			// List all endpoints here.
			.service(hello)
	})
	.bind(bindaddr)?
	.run()
	.await
}
