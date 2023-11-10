use crate::{
	jwt_auth,
	model::{FilterOptions, User, UsersCount},
	response::FilteredUser,
	AppState,
};
use actix_web::{
	get,
	web::{self, Path},
	HttpResponse, Responder,
};
use actix_web_grants::proc_macro::has_any_role;
use serde_json::json;
use uuid::Uuid;

use crate::controllers;
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

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"user": filter_user_record(&user)
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/users")]
#[has_any_role("controllers::auth::Role::Admin", type = "controllers::auth::Role")]
async fn get_users_handler(
	opts: web::Query<FilterOptions>,
	data: web::Data<AppState>,
	_: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let limit = opts.limit.unwrap_or(10);
	let offset = (opts.page.unwrap_or(1) - 1) * limit;

	let query_result = sqlx::query_as!(
		User,
		"SELECT * FROM users ORDER by id LIMIT $1 OFFSET $2",
		limit as i32,
		offset as i32
	)
	.fetch_all(&data.db)
	.await;

	let count_query_result = sqlx::query_as!(UsersCount, "SELECT count(*) AS count FROM users")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		let message = "Что-то пошло не так во время подсчета пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let user_count = count_query_result.unwrap();

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let users = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"users": &users.into_iter().map(|user| filter_user_record(&user)).collect::<Vec<FilteredUser>>(),
			"users_count": &user_count.count.unwrap()
		})
	});

	HttpResponse::Ok().json(json_response)
}
