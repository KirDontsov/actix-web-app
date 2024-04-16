use crate::{
	models::{FilterOptions, OAIDescription},
	AppState,
};
use actix_web::{
	get,
	web::{self, Path},
	HttpResponse, Responder,
};
use serde_json::json;
use uuid::Uuid;

use crate::utils::filter_oai_description_record;

#[get("/oai_description_by_firm/{id}")]
async fn get_oai_description_by_firm_handler(
	path: Path<Uuid>,
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let firm_id = &path.into_inner();
	let table = String::from("oai_descriptions");

	let query_result = sqlx::query_as!(
		OAIDescription,
		"SELECT * FROM oai_descriptions WHERE firm_id = $1",
		firm_id,
	)
	.fetch_one(&data.db)
	.await;

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let oai_description = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"oai_description": filter_oai_description_record(&oai_description),
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/oai_description/{id}")]
async fn get_oai_description_by_id_handler(
	path: Path<Uuid>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let description_id = &path.into_inner();

	let description = sqlx::query_as!(
		OAIDescription,
		"SELECT * FROM oai_descriptions WHERE oai_description_id = $1",
		description_id
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"oai_description": filter_oai_description_record(&description)
		})
	});

	HttpResponse::Ok().json(json_response)
}
