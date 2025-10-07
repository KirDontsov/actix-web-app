use crate::controllers::auth::Role;
use crate::utils::avito_requests::filter_add_avito_request_record;
use crate::{
	jwt_auth::JwtMiddleware,
	models::{
		AdRecord, AvitoRequest, Count, FilterOptions, FilteredAvitoRequest, SaveAvitoRequest,
	},
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

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AvitoRequestMessage {
	pub request_id: Uuid,
	pub user_id: Uuid,
	pub request: String,
	pub city: String,
	pub coords: String,
	pub radius: String,
	pub district: String,
	pub created_ts: chrono::DateTime<chrono::Utc>,
}

#[get("/avito_requests/{id}")]
#[has_any_role("Role::Admin", type = "Role")]
async fn get_avito_requests_handler(
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
async fn create_avito_request_handler(
	path: Path<Uuid>,
	body: web::Json<SaveAvitoRequest>,
	data: web::Data<AppState>,
	_: JwtMiddleware,
) -> impl Responder {
	let user_id = path.into_inner();
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
			// Create message
			let message = AvitoRequestMessage {
				request_id: avito_request.request_id.clone(),
				user_id: avito_request.user_id.clone(),
				request: avito_request.request.clone().expect("REASON"),
				city: avito_request.city.clone().expect("REASON"),
				coords: avito_request.coords.clone().expect("REASON"),
				radius: avito_request.radius.clone().expect("REASON"),
				district: avito_request.district.clone().expect("REASON"),
				created_ts: avito_request.created_ts.clone().expect("REASON"),
			};

			// Publish to RabbitMQ
			match publish_avito_request(&data.rabbitmq_channel, &message).await {
				Ok(_) => {
					let avito_request_response = serde_json::json!({
						"status": "success",
						"data": serde_json::json!({
							"avito_request": filter_add_avito_request_record(&avito_request.clone())
						})
					});
					HttpResponse::Ok().json(avito_request_response)
				}
				Err(e) => {
					log::error!("Failed to publish message: {}", e);
					// You might want to handle this differently - maybe still return success
					// but log the error, or return a partial success response
					HttpResponse::Accepted().json(serde_json::json!({
						"status": "success",
						"message": "Request created but notification failed"
					}))
				}
			}
		}
		Err(e) => HttpResponse::InternalServerError()
			.json(serde_json::json!({"status": "error","message": format!("{:?}", e)})),
	}
}

// Message publishing function
async fn publish_avito_request(
	channel: &lapin::Channel,
	message: &AvitoRequestMessage,
) -> Result<(), Box<dyn std::error::Error>> {
	let message_json = serde_json::to_string(message)?;

	channel
		.basic_publish(
			"",
			"avito_requests",
			lapin::options::BasicPublishOptions::default(),
			message_json.as_bytes(),
			lapin::BasicProperties::default(),
		)
		.await?;

	log::info!(
		"Published Avito request message for user: {}",
		message.user_id
	);
	Ok(())
}

#[get("/avito_requests/{avito_request_id}/ads")]
#[has_any_role("Role::Admin", type = "Role")]
async fn get_ads_by_avito_request_id_handler(
	path: Path<Uuid>,
	data: web::Data<AppState>,
	_: JwtMiddleware,
) -> impl Responder {
	let avito_request_id = path.into_inner();

	let query = "
	           SELECT ad_id, my_ad, run_date, city_query, search_query, position, views, views_today,
	                  promotion, delivery, ad_date, avito_ad_id, title, price, link, categories,
	                  seller_id, seller_name, seller_type, register_date, answer_time,
	                  rating, reviews_count, ads_count, closed_ads_count, photo_count,
	                  address, description, avito_request_id, created_ts
	           FROM avito_analytics_ads
	           WHERE avito_request_id = $1
	           ORDER BY position
	       ";

	let query_result = sqlx::query_as::<_, AdRecord>(query)
		.bind(avito_request_id)
		.fetch_all(&data.db)
		.await;

	match query_result {
		Ok(ads) => {
			let json_response = serde_json::json!({
				"status": "success",
				"data": serde_json::json!({
					"ads": &ads
				})
			});
			HttpResponse::Ok().json(json_response)
		}
		Err(e) => HttpResponse::InternalServerError()
			.json(serde_json::json!({"status": "error","message": format!("{:?}", e)})),
	}
}

#[get("/avito_requests")]
#[has_any_role("Role::Admin", type = "Role")]
async fn get_all_avito_requests_handler(
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	_: JwtMiddleware,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;
	let table = String::from("avito_requests");

	let query_result = sqlx::query_as!(
		AvitoRequest,
		"SELECT * FROM avito_requests ORDER BY created_ts DESC LIMIT $1 OFFSET $2",
		limit as i64,
		offset as i64
	)
	.fetch_all(&data.db)
	.await;

	let error_message = "Error fetching avito requests";
	if query_result.is_err() {
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error", "message": error_message}));
	}

	let avito_requests = query_result.expect(error_message);
	let avito_requests_count = Count::count(&data.db, table).await.unwrap_or(0);

	let json_response = json!({
		"status": "success",
		"data": json!({
			"avito_requests": &avito_requests.into_iter().map(|request| filter_add_avito_request_record(&request)).collect::<Vec<FilteredAvitoRequest>>(),
			"avito_requests_count": &avito_requests_count,
		})
	});

	HttpResponse::Ok().json(json_response)
}
