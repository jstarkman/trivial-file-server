use actix_multipart::Multipart;
use actix_web as aw;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use futures::{StreamExt, TryStreamExt};
use log::{debug, info};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
struct HelloInputs {
	name: String,
}

#[get("/{name}")]
async fn hello(inputs: web::Path<HelloInputs>) -> impl Responder {
	debug!("Called hello.");
	HttpResponse::Ok().body(format!("Hello, {}!", inputs.name))
}

/// Assumes MIME of "multipart/form-data" with names of "identifier" and "f".
/// @see https://github.com/actix/examples/blob/master/forms/multipart/src/main.rs
#[post("/upload")]
async fn upload(mut multipart: Multipart) -> Result<HttpResponse, aw::Error> {
	use actix_http::error::ErrorBadRequest;
	let mut identifier: Option<String> = None;
	let mut filename: Option<String> = None;
	let mut filepath: Option<String> = None;
	// https://docs.rs/futures/0.3/futures/stream/trait.TryStreamExt.html#method.try_next
	while let Ok(Some(mut field)) = multipart.try_next().await {
		// FIXME handle errors
		let content_disposition = field
			.content_disposition()
			.ok_or(ErrorBadRequest("Malformed multipart/form-data header."))?;
		let name = content_disposition
			.get_name()
			.ok_or(ErrorBadRequest("Missing multipart name."))?;
		match name {
			"f" => {
				filename = content_disposition
					.get_filename()
					.map(|strstr| strstr.to_string());
				filepath = Some(stream_to_file(field).await?);
			}
			"identifier" => {
				identifier = match field.next().await {
					Some(r) => match r {
						Ok(bytes) => String::from_utf8(bytes.to_vec()).ok(),
						_ => None,
					},
					_ => None,
				};
			}
			_ => (),
		}
	}
	match (identifier, filename, filepath) {
		(Some(id), Some(fname), Some(fpath)) => {
			info!(
				"Someone uploaded '{}' to '{}'; original filename was '{}'",
				id, fpath, fname
			);
			Ok(aw::HttpResponse::Found()
				// FIXME deprecated; will change in actix-web >= 4.0.0
				.header(aw::http::header::LOCATION, "/static/upload.html")
				.body(""))
		}
		_ => Ok(HttpResponse::from_error(ErrorBadRequest(
			"Incomplete form.",
		))),
	}
}

async fn stream_to_file(mut field: actix_multipart::Field) -> Result<String, aw::Error> {
	use std::fs::File;
	use std::io::Write;
	use std::path::Path;
	// FIXME this should go in application state/config; live env lookup could be expensive
	let tfs_upload = std::env::var("TFS_UPLOAD").unwrap_or("./upload".to_string());
	let filename = Uuid::new_v4().to_hyphenated().to_string();
	let write_here = Path::new(&tfs_upload).join(&filename);
	let mut f: File = aw::web::block(|| File::create(write_here)).await.unwrap();
	while let Some(chunk) = field.next().await {
		let data: aw::web::Bytes = chunk.unwrap();
		f = aw::web::block(move ||  // move to take ownership of `data`
			f.write_all(&data).map(|_| f))
		.await?;
	}
	Ok(filename)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	dotenv::dotenv().ok();
	env_logger::builder().format_timestamp_millis().init();
	let user = std::env::var("CONFIGURED_USER").unwrap_or("unknown user".to_string());
	info!("Hello, {}!", user);

	let static_files_root = std::env::var("TFS_STATIC").unwrap_or("static".to_string());
	let upload_files_root = std::env::var("TFS_UPLOAD").unwrap_or("./upload".to_string());
	std::fs::create_dir_all(upload_files_root)?;

	let bindaddr = std::env::var("TFS_BIND_ADDR").unwrap_or("127.0.0.1:8080".to_string());
	HttpServer::new(move || {
		App::new()
			.service(actix_files::Files::new("/static", &static_files_root))
			.service(
				// List all endpoints here.
				web::scope("api").service(hello).service(upload),
			)
	})
	.bind(bindaddr)?
	.run()
	.await
}
