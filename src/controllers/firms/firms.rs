use crate::{
	models::{
		ExtFilteredFirmWithOaiDescription, ExtFirmWithOaiDescription, FilterOptions, FirmsCount,
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
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;

	let query_result = sqlx::query_as!(
		ExtFirmWithOaiDescription, "SELECT a.firm_id, a.name, a.address, a.site, a.default_phone, a.description, b.oai_description_value FROM firms a 
		JOIN oai_descriptions b ON a.firm_id = b.firm_id
		ORDER BY a.two_gis_firm_id
	 	LIMIT $1 OFFSET $2",
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let count_query_result = sqlx::query_as!(FirmsCount, "SELECT count(*) AS count FROM firms")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		let message = "Что-то пошло не так во время подсчета пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let firm_count = count_query_result.unwrap();

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
			"firms_count": &firm_count.count.unwrap()
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/firm/{id}")]
async fn get_firm_handler(path: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
	let firm_id = &path.into_inner();

	let firm = sqlx::query_as!(ExtFirmWithOaiDescription, "SELECT a.firm_id, a.name, a.address, a.site, a.default_phone, a.description, b.oai_description_value FROM firms a 
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
