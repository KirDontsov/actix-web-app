use crate::{
	jwt_auth,
	models::{FilteredReview, Review, ReviewsCount, ReviewsFilterOptions},
	AppState,
};
use actix_web::{
	get,
	web::{self, Path},
	HttpResponse, Responder,
};
use serde_json::json;
use uuid::Uuid;

use crate::utils::filter_review_record;

#[get("/reviews")]
async fn get_reviews_handler(
	opts: web::Query<ReviewsFilterOptions>,
	data: web::Data<AppState>,
	_: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let firm_id =
		uuid::Uuid::parse_str(opts.firm_id.clone().unwrap_or("".to_string()).as_str()).unwrap();
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;

	let query_result = sqlx::query_as!(
		Review,
		"SELECT * FROM reviews WHERE firm_id = $1 ORDER by review_id LIMIT $2 OFFSET $3",
		firm_id,
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let count_query_result = sqlx::query_as!(ReviewsCount, "SELECT count(*) AS count FROM reviews")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		let message = "Что-то пошло не так во время подсчета пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let review_count = count_query_result.unwrap();

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let reviews = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"reviews": &reviews.into_iter().map(|review| filter_review_record(&review)).collect::<Vec<FilteredReview>>(),
			"reviews_count": &review_count.count.unwrap()
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/review/{id}")]
async fn get_review_handler(
	path: Path<Uuid>,
	data: web::Data<AppState>,
	_: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let review_id = &path.into_inner();

	let review = sqlx::query_as!(
		Review,
		"SELECT * FROM reviews WHERE review_id = $1",
		review_id
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"review": filter_review_record(&review)
		})
	});

	HttpResponse::Ok().json(json_response)
}
