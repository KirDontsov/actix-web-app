use crate::{
	models::{
		Count, ExtFilteredFirmWithOaiDescription, ExtFirmWithOaiDescription, FilterExtOptions,
		FilterOptions,
	},
	utils::filter_firm_record::filter_ext_firm_record,
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
	let type_id =
		uuid::Uuid::parse_str(opts.type_id.clone().unwrap_or("".to_string()).as_str()).unwrap();
	let table = String::from("firms");

	let query_result = sqlx::query_as!(
		ExtFirmWithOaiDescription, "SELECT a.firm_id, a.city_id, a.category_id, a.type_id, a.name, a.address, a.site, a.default_phone, a.description, b.oai_description_value FROM firms a 
		JOIN oai_descriptions b ON a.firm_id = b.firm_id
		WHERE city_id = $1
		AND category_id = $2
		AND type_id = $3
		ORDER BY a.two_gis_firm_id
	 	LIMIT $4 OFFSET $5",
		city_id,
		category_id,
		type_id,
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let firms_count =
		Count::count_firms_by_city_category_type(&data.db, table, city_id, category_id, type_id)
			.await
			.unwrap_or(0);

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let firms = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"firms": &firms.into_iter().map(|firm| filter_ext_firm_record(&firm)).collect::<Vec<ExtFilteredFirmWithOaiDescription>>(),
			"firms_count": &firms_count
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/firm/{id}")]
async fn get_firm_handler(path: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
	let firm_id = &path.into_inner();

	let firm = sqlx::query_as!(ExtFirmWithOaiDescription, "SELECT a.firm_id, a.city_id, a.category_id, a.type_id, a.name, a.address, a.site, a.default_phone, a.description, b.oai_description_value FROM firms a 
		JOIN oai_descriptions b ON a.firm_id = b.firm_id
		WHERE a.firm_id = $1", firm_id)
	.fetch_one(&data.db)
	.await
	.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"firm": filter_ext_firm_record(&firm)
		})
	});

	HttpResponse::Ok().json(json_response)
}
