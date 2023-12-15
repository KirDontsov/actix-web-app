use crate::{
	jwt_auth,
	models::{AddQuoteSchema, FilterOptions, FilteredQuote, Quote, QuotesCount},
	AppState,
};
use actix_web::{
	delete, get, post, put,
	web::{self, Path},
	HttpResponse, Responder,
};
use actix_web_grants::proc_macro::has_any_role;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::controllers::auth::Role;
use crate::utils::filter_quote_record;

#[get("/quotes")]
async fn get_quotes_handler(
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;

	let query_result = sqlx::query_as!(
		Quote,
		"SELECT * FROM quotes ORDER by id LIMIT $1 OFFSET $2",
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let count_query_result = sqlx::query_as!(QuotesCount, "SELECT count(*) AS count FROM quotes")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		let message = "Что-то пошло не так во время подсчета пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let quote_count = count_query_result.unwrap();

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let quotes = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"quotes": &quotes.into_iter().map(|quote| filter_quote_record(&quote)).collect::<Vec<FilteredQuote>>(),
			"quotes_count": &quote_count.count.unwrap()
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/quote/{id}")]
async fn get_quote_handler(
	path: Path<Uuid>,
	data: web::Data<AppState>,
	_: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let quote_id = &path.into_inner();

	let quote = sqlx::query_as!(Quote, "SELECT * FROM quotes WHERE id = $1", quote_id)
		.fetch_one(&data.db)
		.await
		.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"quote": filter_quote_record(&quote)
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[post("/quote")]
async fn add_quote_handler(
	body: web::Json<AddQuoteSchema>,
	data: web::Data<AppState>,
) -> impl Responder {
	let query_result = sqlx::query_as!(
		Quote,
		"INSERT INTO quotes (text,author) VALUES ($1, $2) RETURNING *",
		body.text.to_string().to_lowercase(),
		body.author.to_string().to_lowercase(),
	)
	.fetch_one(&data.db)
	.await;

	match query_result {
		Ok(quote) => {
			let quote_response = serde_json::json!({"status": "success","data": serde_json::json!({
				"quote": filter_quote_record(&quote)
			})});

			return HttpResponse::Ok().json(quote_response);
		}
		Err(e) => {
			return HttpResponse::InternalServerError()
				.json(serde_json::json!({"status": "error","message": format!("{:?}", e)}));
		}
	}
}
