use crate::{jwt_auth, model::User, response::FilteredUser, AppState};
use actix_web::{
	get,
	web::{self, Path},
	HttpResponse, Responder,
};
use uuid::Uuid;

use crate::utils::filter_user_record;

#[get("/user/{id}")]
async fn get_user_handler(
	path: Path<Uuid>,
	data: web::Data<AppState>,
	_: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let user_id = &path.into_inner();

	let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
		.fetch_one(&data.db)
		.await
		.unwrap();

	let json_response = serde_json::json!({
		"status":  "success",
		"data": serde_json::json!({
			"user": filter_user_record(&user)
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/users")]
async fn get_users_handler(
	data: web::Data<AppState>,
	_: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let users = sqlx::query_as!(User, "SELECT * FROM users")
		.fetch_all(&data.db)
		.await
		.unwrap();

	let json_response = serde_json::json!({
		"status":  "success",
		"data": serde_json::json!({
			"users": &users.into_iter().map(|user| filter_user_record(&user)).collect::<Vec<FilteredUser>>()
		})
	});

	HttpResponse::Ok().json(json_response)
}
