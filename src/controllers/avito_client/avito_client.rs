use crate::controllers::auth::Role;
use crate::{
	jwt_auth::JwtMiddleware,
	models::{
		FilterExtOptions,
	},
	AppState,
};
use actix_web::{
	post,
	web::{self, Path},
	HttpResponse, Responder,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

use reqwest::header::{self, HeaderMap, HeaderValue};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AvitoTokenCredentials {
	pub client_id: String,
	pub client_secret: String,
	pub grant_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AvitoTokenResponse {
	access_token: String,
	token_type: String,
	expires_in: i64,
}

#[post("/avito/get_token")]
pub async fn get_avito_token_handler(
	opts: web::Query<FilterExtOptions>,
	data: web::Data<AppState>,
) -> impl Responder {
	let url = env::var("AVITO_BASE_URL").expect("AVITO_BASE_URL not set");
	let client_id = env::var("AVITO_CLIENT_ID").expect("AVITO_BASE_URL not set");
	let client_secret = env::var("AVITO_CLIENT_SECRET").expect("AVITO_BASE_URL not set");

	let headers: HeaderMap<HeaderValue> = header::HeaderMap::from_iter(vec![(
		header::CONTENT_TYPE,
		"application/x-www-form-urlencoded".parse().unwrap(),
	)]);

	let body = AvitoTokenCredentials {
		client_id,
		client_secret,
		grant_type: String::from("client_credentials"),
	};

	// request
	let token_query_result = reqwest::Client::builder()
		.danger_accept_invalid_certs(true)
		.build()
		.unwrap()
		.post(format!("{}/token", url.clone()))
		.headers(headers)
		.form(&body)
		.send()
		.await
		.map_err(|e| {
			HttpResponse::BadGateway().json(json!({
			  "status": "error",
			  "message": format!("Avito API request failed: {}", e)
			}))
		});

	let response = match token_query_result {
		Ok(res) => res,
		Err(e) => {
			return HttpResponse::BadGateway().json(json!({
				"status": "error",
				"message": "Avito API request failed",
			}))
		}
	};

	// Check response status
	if !response.status().is_success() {
		let error_body = match response.text().await {
			Ok(body) => body,
			Err(e) => format!("Failed to read error body: {}", e),
		};
		return HttpResponse::BadRequest().json(json!({
			"status": "error",
			"message": format!("Avito API error: {}", error_body)
		}));
	};

	// Parse the response text into our struct
	let token_text = match response.text().await {
		Ok(text) => text,
		Err(e) => {
			return HttpResponse::InternalServerError().json(json!({
				"status": "error",
				"message": format!("Failed to read token response: {}", e)
			}))
		}
	};

	// Now parse the JSON string into our struct
	let token_data: AvitoTokenResponse = match serde_json::from_str(&token_text) {
		Ok(data) => data,
		Err(e) => {
			return HttpResponse::InternalServerError().json(json!({
				"status": "error",
				"message": format!("Failed to parse token JSON: {}", e),
				"raw_response": token_text
			}))
		}
	};

	dbg!(&token_data);

	HttpResponse::Ok().json(json!({
		"status": "success",
		"data": {
			"access_token": token_data.access_token,
			"token_type": token_data.token_type,
			"expires_in": token_data.expires_in,
		}
	}))
}

// #[post("/items")]
// pub async fn create_item(
//     pool: web::Data<DbPool>,
//     payload: web::Json<CreateItemRequest>,
// ) -> Result<impl Responder, ApiError> {
//     let item = payload.into_inner();
//     item.validate()?;

//     let mut conn = pool.acquire().await?;
//     let new_item = sqlx::query!(
//         r#"
//         INSERT INTO items (avito_item_id, price_cents, currency)
//         VALUES ($1, $2, $3)
//         RETURNING id, avito_item_id, price_cents, currency, updated_at
//         "#,
//         item.avito_item_id,
//         item.price_cents,
//         item.currency
//     )
//     .fetch_one(&mut *conn)
//     .await?;

//     let response = ItemResponse::from(new_item);
//     Ok(HttpResponse::Created().json(response))
// }

// #[get("/items/{id}")]
// pub async fn get_item(
//     pool: web::Data<DbPool>,
//     path: web::Path<Uuid>,
// ) -> Result<impl Responder, ApiError> {
//     let id = path.into_inner();
//     let mut conn = pool.acquire().await?;

//     let item = sqlx::query!(
//         r#"
//         SELECT id, avito_item_id, price_cents, currency, updated_at
//         FROM items
//         WHERE id = $1
//         "#,
//         id
//     )
//     .fetch_optional(&mut *conn)
//     .await?;

//     match item {
//         Some(item) => {
//             let response = ItemResponse::from(item);
//             Ok(HttpResponse::Ok().json(response))
//         }
//         None => Err(ApiError::NotFound("Item not found".into())),
//     }
// }

// #[post("/items/{id}/price")]
// pub async fn update_item_price(
//     pool: web::Data<DbPool>,
//     path: web::Path<Uuid>,
//     payload: web::Json<PriceUpdateRequest>,
//     avito_client: web::Data<AvitoClient>,
// ) -> Result<impl Responder, ApiError> {
//     let id = path.into_inner();
//     let new_price = payload.into_inner();
//     new_price.validate()?;

//     let mut tx = pool.begin().await?;

//     // Fetch item with FOR UPDATE lock
//     let item = sqlx::query!(
//         r#"
//         SELECT id, avito_item_id, price_cents, currency
//         FROM items
//         WHERE id = $1
//         FOR UPDATE
//         "#,
//         id
//     )
//     .fetch_optional(&mut *tx)
//     .await?;

//     let item = match item {
//         Some(item) => item,
//         None => return Err(ApiError::NotFound("Item not found".into())),
//     };

//     // Update local DB
//     sqlx::query!(
//         r#"
//         UPDATE items
//         SET price_cents = $1, updated_at = NOW()
//         WHERE id = $2
//         "#,
//         new_price.price_cents,
//         id
//     )
//     .execute(&mut *tx)
//     .await?;

//     // Update on Avito
//     let price_update = AvitoPriceUpdate {
//         price: new_price.price_cents,
//         currency: item.currency,
//     };

//     match avito_client.update_price(&item.avito_item_id, price_update).await {
//         Ok(_) => {
//             tx.commit().await?;
//             let updated_item = sqlx::query!(
//                 r#"
//                 SELECT id, avito_item_id, price_cents, currency, updated_at
//                 FROM items
//                 WHERE id = $1
//                 "#,
//                 id
//             )
//             .fetch_one(&mut *pool.acquire().await?)
//             .await?;

//             let response = ItemResponse::from(updated_item);
//             Ok(HttpResponse::Ok().json(response))
//         }
//         Err(e) => {
//             tx.rollback().await?;
//             Err(e)
//         }
//     }
// }
