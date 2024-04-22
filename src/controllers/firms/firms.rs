use crate::{
	models::{Count, FilterExtOptions, FilteredFirm, Firm, City, Category},
	utils::filter_firm_record::filter_firm_record,
	AppState,
};
use actix_web::{
	get,
	web::{self, Path},
	HttpResponse, Responder,
};
use serde_json::json;
use uuid::Uuid;

#[get("/firms")]
async fn get_firms_handler(
	opts: web::Query<FilterExtOptions>,
	data: web::Data<AppState>,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;
	let city_id =
		uuid::Uuid::parse_str(opts.city_id.clone().unwrap_or("".to_string()).as_str()).unwrap();
	let category_id =
		uuid::Uuid::parse_str(opts.category_id.clone().unwrap_or("".to_string()).as_str()).unwrap();
	let table = String::from("firms");

	let query_result = sqlx::query_as!(
		Firm,
		"SELECT * FROM firms
		WHERE city_id = $1
		AND category_id = $2
		ORDER BY two_gis_firm_id
	 	LIMIT $3 OFFSET $4",
		city_id,
		category_id,
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let firms_count = Count::count_firms_by_city_category(&data.db, table, city_id, category_id)
		.await
		.unwrap_or(0);

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения firms";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let firms = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"firms": &firms.into_iter().map(|firm| filter_firm_record(&firm)).collect::<Vec<FilteredFirm>>(),
			"firms_count": &firms_count
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/firms_by_abbr")]
async fn get_firms_by_abbr_handler(
	opts: web::Query<FilterExtOptions>,
	data: web::Data<AppState>,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;
	let city_id = opts.city_id.clone().unwrap_or("".to_string());
	let category_id = opts.category_id.clone().unwrap_or("".to_string());
	let table = String::from("firms");

	let city_query_result = sqlx::query_as!(
		City,
		"SELECT * FROM cities
		WHERE abbreviation = $1
		",
		city_id,
	)
	.fetch_one(&data.db)
	.await;

	let city = city_query_result.unwrap();

	let category_query_result = sqlx::query_as!(
		Category,
		"SELECT * FROM categories
		WHERE abbreviation = $1
		",
		category_id,
	)
	.fetch_one(&data.db)
	.await;

	let category = category_query_result.unwrap();

	let query_result = sqlx::query_as!(
		Firm,
		"SELECT * FROM firms
		WHERE city_id = $1
		AND category_id = $2
		ORDER BY two_gis_firm_id
	 	LIMIT $3 OFFSET $4",
		city.city_id.clone(), category.category_id.clone(),
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let firms_count = Count::count_firms_by_city_category(&data.db, table, city.city_id.clone(), category.category_id.clone())
		.await
		.unwrap_or(0);

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения firms";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let firms = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"firms": &firms.into_iter().map(|firm| filter_firm_record(&firm)).collect::<Vec<FilteredFirm>>(),
			"firms_count": &firms_count
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/firm/{id}")]
async fn get_firm_handler(path: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
	let firm_id = &path.into_inner();

	let firm = sqlx::query_as!(Firm, "SELECT * FROM firms WHERE firm_id = $1", firm_id)
		.fetch_one(&data.db)
		.await
		.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"firm": filter_firm_record(&firm)
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/firm_url/{id}")]
async fn get_firm_by_url_handler(path: Path<String>, data: web::Data<AppState>) -> impl Responder {
	let firm_url = &path.into_inner();

	let firm = sqlx::query_as!(Firm, "SELECT * FROM firms WHERE url = $1", firm_url)
		.fetch_one(&data.db)
		.await
		.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"firm": filter_firm_record(&firm)
		})
	});

	HttpResponse::Ok().json(json_response)
}
