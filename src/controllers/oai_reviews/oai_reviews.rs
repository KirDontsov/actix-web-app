use crate::{
	models::{Count, FilterOptions, FilteredOAIReview, OAIReview, Firm},
	AppState,
};
use actix_web::{
	get,
	web::{self, Path},
	HttpResponse, Responder,
};
use serde_json::json;
use uuid::Uuid;

use crate::utils::filter_oai_review_record;

#[get("/oai_reviews/{id}")]
async fn get_oai_reviews_handler(
	path: Path<Uuid>,
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let firm_id = &path.into_inner();
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;
	let table = String::from("oai_reviews");

	let query_result = sqlx::query_as!(
		OAIReview,
		"SELECT * FROM oai_reviews WHERE firm_id = $1 ORDER by oai_review_id LIMIT $2 OFFSET $3",
		firm_id,
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let reviews_count = Count::count(&data.db, table).await.unwrap_or(0);

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let oai_reviews = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"oai_reviews": &oai_reviews.into_iter().map(|oai_review| filter_oai_review_record(&oai_review)).collect::<Vec<FilteredOAIReview>>(),
			"oai_reviews_count": &reviews_count
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/oai_review/{id}")]
async fn get_oai_review_handler(
	path: Path<Uuid>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let review_id = &path.into_inner();

	let review = sqlx::query_as!(
		OAIReview,
		"SELECT * FROM oai_reviews WHERE oai_review_id = $1",
		review_id
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"oai_review": filter_oai_review_record(&review)
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/oai_reviews_by_url/{id}")]
async fn get_oai_reviews_by_url_handler(
	path: Path<String>,
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;
	let table = String::from("oai_reviews");

	let firm_url = &path.into_inner();
	let firm_query_result = Firm::get_firm_by_url(&data.db, &firm_url).await;
	let firm = firm_query_result.unwrap();
	let firm_id = firm.firm_id;

	let query_result = sqlx::query_as!(
		OAIReview,
		"SELECT * FROM oai_reviews WHERE firm_id = $1 ORDER by oai_review_id LIMIT $2 OFFSET $3",
		firm_id,
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let reviews_count = Count::count(&data.db, table).await.unwrap_or(0);

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let oai_reviews = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"oai_reviews": &oai_reviews.into_iter().map(|oai_review| filter_oai_review_record(&oai_review)).collect::<Vec<FilteredOAIReview>>(),
			"oai_reviews_count": &reviews_count
		})
	});

	HttpResponse::Ok().json(json_response)
}
