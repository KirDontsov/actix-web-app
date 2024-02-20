use crate::{
	models::{FilteredImage, Image, ImageCount},
	utils::filter_image_record::filter_image_record,
	AppState,
};
use actix_web::{
	get,
	web::{self, Path},
	HttpResponse, Responder,
};
use serde_json::json;
use uuid::Uuid;

#[get("/images/{id}")]
async fn get_images_handler(path: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
	let firm_id = &path.into_inner();

	let query_result = sqlx::query_as!(Image, "SELECT * FROM images WHERE firm_id = $1", firm_id)
		.fetch_all(&data.db)
		.await;

	let count_query_result = sqlx::query_as!(ImageCount, "SELECT count(*) AS count FROM images")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		let message = "Что-то пошло не так во время подсчета пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let image_count = count_query_result.unwrap();

	if query_result.is_err() {
		let message = "Что-то пошло не так во время чтения пользователей";
		return HttpResponse::InternalServerError()
			.json(json!({"status": "error","message": message}));
	}

	let images = query_result.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"images": &images.into_iter().map(|image| filter_image_record(&image)).collect::<Vec<FilteredImage>>(),
			"images_count": &image_count.count.unwrap()
		})
	});

	HttpResponse::Ok().json(json_response)
}

#[get("/image/{id}")]
async fn get_image_handler(path: Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
	let img_id = &path.into_inner();

	let image = sqlx::query_as!(Image, "SELECT * FROM images WHERE img_id = $1", img_id)
		.fetch_one(&data.db)
		.await
		.unwrap();

	let json_response = json!({
		"status":  "success",
		"data": json!({
			"image": filter_image_record(&image)
		})
	});

	HttpResponse::Ok().json(json_response)
}
