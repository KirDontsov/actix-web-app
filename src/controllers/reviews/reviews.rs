use crate::{
	models::{AddReview, Count, FilterOptions, FilteredReview, Review, Firm},
	AppState,
};
use actix_web::{
	get, post,
	web::{self, Path},
	HttpResponse, Responder,
};
use serde_json::json;
use uuid::Uuid;

use crate::utils::{filter_add_review_record, filter_review_record};

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

	let query_result = sqlx::query_as!(
		Review,
		"SELECT * FROM reviews WHERE firm_id = $1 ORDER by review_id LIMIT $2 OFFSET $3",
		firm_id,
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let count_query_result = sqlx::query_as!(
		Count,
		"SELECT count(*) AS count FROM reviews WHERE firm_id = $1",
		firm_id
	)
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

#[get("/reviews_by_url/{id}")]
async fn get_reviews_by_url_handler(
	path: Path<String>,
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;

	let firm_url = &path.into_inner();
	let firm_query_result = Firm::get_firm_by_url(&data.db, &firm_url).await;
	let firm = firm_query_result.unwrap();
	let firm_id = firm.firm_id;

	let query_result = sqlx::query_as!(
		Review,
		"SELECT * FROM reviews WHERE firm_id = $1 ORDER by review_id LIMIT $2 OFFSET $3",
		firm_id,
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let count_query_result = sqlx::query_as!(
		Count,
		"SELECT count(*) AS count FROM reviews WHERE firm_id = $1",
		firm_id
	)
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

#[post("/review/{id}")]
async fn add_review_handler(
	body: web::Json<AddReview>,
	data: web::Data<AppState>,
) -> impl Responder {
	let firm_id = uuid::Uuid::parse_str(body.firm_id.to_string().as_str()).unwrap();
	dbg!(&firm_id);
	let query_result = sqlx::query_as!(
		Review,
		"INSERT INTO reviews (firm_id, text, author, rating, parsed) VALUES ($1, $2, $3, $4, $5) RETURNING *",
		firm_id,
		body.text.to_string().to_lowercase(),
		body.author.to_string().to_lowercase(),
		body.rating.to_string().to_lowercase(),
		false
	)
	.fetch_one(&data.db)
	.await;

	match query_result {
		Ok(review) => {
			let review_response = serde_json::json!({"status": "success","data": serde_json::json!({
				"review": filter_add_review_record(&review)
			})});

			return HttpResponse::Ok().json(review_response);
		}
		Err(e) => {
			return HttpResponse::InternalServerError()
				.json(serde_json::json!({"status": "error","message": format!("{:?}", e)}));
		}
	}
}
