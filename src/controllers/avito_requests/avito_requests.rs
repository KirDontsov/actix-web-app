use crate::controllers::auth::Role;
use crate::utils::avito_requests::filter_add_avito_request_record;
use crate::{
	jwt_auth::JwtMiddleware,
	models::{AvitoRequest, Count, FilterOptions, FilteredAvitoRequest, SaveAvitoRequest},
	AppState,
};
use actix_web::{
	get, post,
	web::{self, Path},
	HttpResponse, Responder,
};
use actix_web_grants::proc_macro::has_any_role;
use serde_json::json;
use uuid::Uuid;

#[get("/avito_requests/{id}")]
#[has_any_role("Role::Admin", type = "Role")]
async fn get_avito_requsts_handler(
	path: Path<Uuid>,
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let user_id = &path.into_inner();
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;
	let table = String::from("avito_requests");

	let query_result =
		AvitoRequest::get_avito_requests_by_user(&data.db, user_id, limit as i64, offset as i64)
			.await;
	let reviews_message = "Что-то пошло не так во время чтения category";
	if query_result.is_err() {
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": &reviews_message}));
	}
	let reviews = query_result.expect(&reviews_message);

	let avito_requests_count = Count::count(&data.db, table).await.unwrap_or(0);

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"avito_requests": &reviews.into_iter().map(|review| filter_add_avito_request_record(&review)).collect::<Vec<FilteredAvitoRequest>>(),
			"avito_requests_count": &avito_requests_count,
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[post("/avito_requests/{id}")]
#[has_any_role("Role::Admin", type = "Role")]
async fn create_avito_requst_handler(
	path: Path<Uuid>,
	body: web::Json<SaveAvitoRequest>,
	data: web::Data<AppState>,
	_: JwtMiddleware,
) -> impl Responder {
	let user_id = &path.into_inner();
	let request = &body.request;
	let city = &body.city;
	let coords = &body.coords;
	let radius = &body.radius;
	let district = &body.district;

	let query_result = sqlx::query_as!(
		AvitoRequest,
		"INSERT INTO avito_requests (user_id, request, city, coords, radius, district) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
		user_id,
		request.to_string(),
		city.to_string(),
		coords.to_string(),
		radius.to_string(),
		district.to_string(),
	)
	.fetch_one(&data.db)
	.await;

	match query_result {
		Ok(avito_request) => {
			let avito_request_response = serde_json::json!({"status": "success","data": serde_json::json!({
				"avito_request": filter_add_avito_request_record(&avito_request)
			})});

			return HttpResponse::Ok().json(avito_request_response);
		}
		Err(e) => {
			return HttpResponse::InternalServerError()
				.json(serde_json::json!({"status": "error","message": format!("{:?}", e)}));
		}
	}
}
