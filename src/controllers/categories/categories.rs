use crate::{
	models::{CategoriesCount, Category, FilterOptions, FilteredCategory},
	AppState,
};
use actix_web::{
	get,
	web::{self, Path},
	HttpResponse, Responder,
};
use serde_json::json;
use uuid::Uuid;

use crate::utils::filter_category_record;

#[get("/categories")]
async fn get_categories_handler(
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;

	let query_result = sqlx::query_as!(
		Category,
		"SELECT * FROM categories ORDER by category_id LIMIT $1 OFFSET $2",
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let count_query_result =
		sqlx::query_as!(CategoriesCount, "SELECT count(*) AS count FROM categories")
			.fetch_one(&data.db)
			.await;

	if count_query_result.is_err() {
		let message = "Что-то пошло не так во время подсчета categories";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let category_count = count_query_result.unwrap();

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения categories";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let categories = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"categories": &categories.into_iter().map(|category| filter_category_record(&category)).collect::<Vec<FilteredCategory>>(),
			"categories_count": &category_count.count.unwrap()
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/category/{id}")]
async fn get_category_handler(path: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
	let category_id = &path.into_inner();

	let category = sqlx::query_as!(
		Category,
		"SELECT * FROM categories WHERE category_id = $1",
		category_id
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"category": filter_category_record(&category)
		})
	});

	HttpResponse::Ok().json(json_response)
}
