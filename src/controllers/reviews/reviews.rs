use crate::{
	models::{Count, FilterOptions, FilteredReview, Review},
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

#[get("/reviews/{id}")]
async fn get_reviews_handler(
	path: Path<Uuid>,
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let firm_id = &path.into_inner();
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;
	let table = String::from("reviews");

	let query_result = sqlx::query_as!(
		Review,
		"SELECT * FROM reviews WHERE firm_id = $1 ORDER by review_id LIMIT $2 OFFSET $3",
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

	let reviews = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"reviews": &reviews.into_iter().map(|review| filter_review_record(&review)).collect::<Vec<FilteredReview>>(),
			"reviews_count": &reviews_count
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/review/{id}")]
async fn get_review_handler(
	path: Path<Uuid>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
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
