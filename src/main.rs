mod config;
mod controllers;
mod jwt_auth;
mod model;
mod response;
mod utils;

use actix_cors::Cors;
use actix_web::dev::ServiceRequest;
use actix_web::middleware::Logger;
use actix_web::{http, HttpMessage};
use actix_web::{http::header, web, App, Error, HttpServer};
use actix_web_grants::{proc_macro::has_any_role, GrantsMiddleware};
use config::Config;
use controllers::auth::Role;
use dotenv::dotenv;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

use core::fmt;
use std::future::{ready, Ready};

use actix_web::error::ErrorUnauthorized;
use actix_web::{dev::Payload, Error as ActixWebError};
use actix_web::{FromRequest, HttpRequest};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Serialize;

use crate::model::{TokenClaims, User};

use crate::controllers::auth::extract;

#[derive(Debug, Serialize)]
struct ErrorResponse {
	status: String,
	message: String,
}

impl fmt::Display for ErrorResponse {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", serde_json::to_string(&self).unwrap())
	}
}

pub struct AppState {
	db: Pool<Postgres>,
	env: Config,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	if std::env::var_os("RUST_LOG").is_none() {
		std::env::set_var("RUST_LOG", "actix_web=info");
	}
	dotenv().ok();
	env_logger::init();

	let config = Config::init();

	let pool = match PgPoolOptions::new()
		.max_connections(10)
		.connect(&config.database_url)
		.await
	{
		Ok(pool) => {
			println!("✅Connection to the database is successful!");
			pool
		}
		Err(err) => {
			println!("🔥 Failed to connect to the database: {:?}", err);
			std::process::exit(1);
		}
	};

	println!("🚀 Server started successfully");

	HttpServer::new(move || {
		// let cors = Cors::default()
		// 	.allowed_origin("http://localhost:3000")
		// 	.allowed_methods(vec!["GET", "POST"])
		// 	.allowed_headers(vec![
		// 		header::CONTENT_TYPE,
		// 		header::AUTHORIZATION,
		// 		header::ACCEPT,
		// 	])
		// 	.supports_credentials();
		let auth = GrantsMiddleware::with_extractor(extract);
		App::new()
			.app_data(web::Data::new(AppState {
				db: pool.clone(),
				env: config.clone(),
			}))
			.configure(controllers::config)
			.wrap(Cors::permissive())
			.wrap(Logger::default())
			.wrap(auth)
	})
	.bind(("127.0.0.1", 8000))?
	.run()
	.await
}
