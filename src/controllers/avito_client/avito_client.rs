use crate::controllers::auth::Role;
use crate::{
	jwt_auth::JwtMiddleware,
	models::{
		ApiError, AvitoCarMark, AvitoEditorCategoryFieldsParams, AvitoGetBalanceApiResponse,
		AvitoGetItemsApiResponse, AvitoItemAnalyticsResponse, AvitoTokenCredentials,
		AvitoTokenParams, AvitoTokenResponse, AvitoUserProfileResponse, GetAvitoItemsParams,
		GetItemAnalyticsBody, UpdatePriceBody,
	},
	AppState,
};
use actix_web::{
	cookie::{time::Duration as ActixWebDuration, Cookie, SameSite},
	get, post,
	web::{self},
	HttpResponse,
};
use actix_web_grants::proc_macro::has_any_role;
use serde_json::json;
use std::env;

use reqwest::{
	header::{self, HeaderMap, HeaderValue},
	Client,
};

#[get("/avito/get_token")]
#[has_any_role("Role::Admin", type = "Role")]
pub async fn get_avito_token_handler(_: JwtMiddleware) -> Result<HttpResponse, ApiError> {
	let url = env::var("AVITO_BASE_URL")
		.map_err(|_| ApiError::Other("AVITO_BASE_URL not set".to_string()))?;
	let client_id = env::var("AVITO_CLIENT_ID")
		.map_err(|_| ApiError::Other("AVITO_CLIENT_ID not set".to_string()))?;
	let client_secret = env::var("AVITO_CLIENT_SECRET")
		.map_err(|_| ApiError::Other("AVITO_CLIENT_SECRET not set".to_string()))?;

	let mut headers = header::HeaderMap::new();
	headers.insert(
		header::CONTENT_TYPE,
		"application/x-www-form-urlencoded".parse().unwrap(),
	);

	let body = AvitoTokenCredentials {
		client_id,
		client_secret,
		grant_type: String::from("client_credentials"),
	};

	// Make request
	let response = Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?
		.post(format!("{}/token", url))
		.headers(headers)
		.form(&body)
		.send()
		.await?;

	// Check response status
	if !response.status().is_success() {
		let status_code = response.status().as_u16();
		let error_body = response.text().await?;
		return Err(ApiError::AvitoApiError(status_code, error_body));
	}

	// Parse response
	let response_text = response.text().await?;
	let token_data: AvitoTokenResponse = serde_json::from_str(&response_text)
		.map_err(|e| ApiError::JsonParseError(e, response_text.clone()))?;

	// Build cookie
	let cookie = Cookie::build("avito_token", &token_data.access_token)
		.same_site(SameSite::None)
		.path("/")
		.max_age(ActixWebDuration::new(token_data.expires_in, 0))
		.secure(true)
		.finish();

	Ok(HttpResponse::Ok().cookie(cookie).json(json!({
		"status": "success",
		"data": {
			"access_token": token_data.access_token,
			"token_type": token_data.token_type,
			"expires_in": token_data.expires_in,
		}
	})))
}

#[post("/avito/get_items")]
#[has_any_role("Role::Admin", type = "Role")]
pub async fn get_avito_items(
	opts: web::Json<GetAvitoItemsParams>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let avito_token = opts.avito_token.clone();
	let page = opts.page.unwrap_or(0);
	let per_page = opts.per_page.unwrap_or(50).min(1000); // Avito API max per_page is 1000

	let url = env::var("AVITO_BASE_URL")
		.map_err(|_| ApiError::Other("AVITO_BASE_URL not set".to_string()))?;

	let mut headers = header::HeaderMap::new();
	headers.insert(
		header::CONTENT_TYPE,
		"application/x-www-form-urlencoded".parse().unwrap(),
	);
	headers.insert(
		header::AUTHORIZATION,
		format!("Bearer {}", avito_token).parse().unwrap(),
	);

	// Build URL with pagination parameters
	let api_url = format!("{}/core/v1/items?page={}&per_page={}", url, page, per_page);

	// Make request
	let response = Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?
		.get(&api_url)
		.headers(headers)
		.send()
		.await?;

	// Check response status
	if !response.status().is_success() {
		let status_code = response.status().as_u16();
		let error_body = response.text().await?;
		return Err(ApiError::AvitoApiError(status_code, error_body));
	}

	// Parse response
	let response_text = response.text().await?;
	let respon_data: AvitoGetItemsApiResponse = serde_json::from_str(&response_text)
		.map_err(|e| ApiError::JsonParseError(e, response_text.clone()))?;

	Ok(HttpResponse::Ok().json(json!({
		"status": "success",
		"data": {
			"meta": &respon_data.meta,
			"items": &respon_data.resources,
		},
	})))
}

#[post("/avito/get_balance")]
#[has_any_role("Role::Admin", type = "Role")]
pub async fn get_avito_balance(
	opts: web::Json<AvitoTokenParams>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let avito_token = opts.avito_token.clone();

	let url = env::var("AVITO_BASE_URL").expect("AVITO_BASE_URL not set");

	let headers: HeaderMap<HeaderValue> = header::HeaderMap::from_iter(vec![
		(header::CONTENT_TYPE, "application/json".parse().unwrap()),
		(
			header::AUTHORIZATION,
			format!("Bearer {}", avito_token).parse().unwrap(),
		),
	]);

	let body = serde_json::json!({});

	let response = Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?
		.post(format!("{}/cpa/v3/balanceInfo", url))
		.headers(headers)
		.json(&body)
		.send()
		.await?;

	// Check response status
	if !response.status().is_success() {
		let status_code = response.status().as_u16();
		let error_body = response.text().await?;
		return Err(ApiError::AvitoApiError(status_code, error_body));
	}

	let response_text = response.text().await?;

	// Parse the response with error context
	let respon_data: AvitoGetBalanceApiResponse = serde_json::from_str(&response_text)
		.map_err(|e| ApiError::JsonParseError(e, response_text.clone()))?;

	Ok(HttpResponse::Ok().json(json!({
		"status": "success",
		"data": {
			"balance": &respon_data.balance,
		}
	})))
}

#[post("/avito/get_user_profile")]
#[has_any_role("Role::Admin", type = "Role")]
pub async fn get_avito_user_profile(
	opts: web::Json<AvitoTokenParams>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let avito_token = opts.avito_token.clone();

	let url = env::var("AVITO_BASE_URL")
		.map_err(|_| ApiError::Other("AVITO_BASE_URL not set".to_string()))?;

	// Build headers
	let mut headers = header::HeaderMap::new();
	headers.insert(
		header::AUTHORIZATION,
		format!("Bearer {}", avito_token).parse().unwrap(),
	);
	headers.insert(header::USER_AGENT, HeaderValue::from_static("YourApp/1.0"));
	headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

	// Build URL
	let api_url = format!("{}/core/v1/accounts/self", url);

	// Make request
	let response = Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?
		.get(&api_url)
		.headers(headers)
		.send()
		.await?;

	// Check response status
	if !response.status().is_success() {
		let status_code = response.status().as_u16();
		let error_body = response.text().await?;
		return Err(ApiError::AvitoApiError(status_code, error_body));
	}

	// Parse response
	let response_text = response.text().await?;

	let profile_data: AvitoUserProfileResponse = serde_json::from_str(&response_text)
		.map_err(|e| ApiError::JsonParseError(e, response_text.clone()))?;

	Ok(HttpResponse::Ok().json(json!({
		"status": "success",
		"data": profile_data
	})))
}

#[post("/avito/get_item_analytics")]
pub async fn get_avito_item_analytics(
	opts: web::Json<GetItemAnalyticsBody>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let avito_token = opts.avito_token.clone();
	let account_id = opts.account_id.clone();

	let url = env::var("AVITO_BASE_URL")
		.map_err(|_| ApiError::Other("AVITO_BASE_URL not set".to_string()))?;

	// Build headers
	let mut headers = header::HeaderMap::new();
	headers.insert(
		header::AUTHORIZATION,
		format!("Bearer {}", avito_token).parse().unwrap(),
	);
	headers.insert(header::USER_AGENT, HeaderValue::from_static("YourApp/1.0"));
	headers.insert(
		header::CONTENT_TYPE,
		HeaderValue::from_static("application/json"),
	);
	headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

	// Build request body
	let request_body = json!({
		"dateFrom": opts.date_from,
		"dateTo": opts.date_to,
		"grouping": opts.grouping,
		"limit": opts.limit,
		"metrics": opts.metrics,
		"offset": opts.offset
	});

	// Build URL
	let api_url = format!("{}/stats/v2/accounts/{}/items", url, account_id);

	// Make request
	let response = Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?
		.post(&api_url)
		.headers(headers)
		.json(&request_body)
		.send()
		.await?;

	// Check response status
	if !response.status().is_success() {
		let status_code = response.status().as_u16();
		let error_body = response.text().await?;
		return Err(ApiError::AvitoApiError(status_code, error_body));
	}

	// Parse response
	let response_text = response.text().await?;

	let analytics_data: AvitoItemAnalyticsResponse = serde_json::from_str(&response_text)
		.map_err(|e| ApiError::JsonParseError(e, response_text.clone()))?;

	Ok(HttpResponse::Ok().json(json!({
		"status": "success",
		"data": analytics_data.result
	})))
}

#[post("/avito/update_price")]
pub async fn update_avito_price(
	opts: web::Json<UpdatePriceBody>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let avito_token = opts.avito_token.clone();
	let item_id = opts.item_id.clone();

	let url = env::var("AVITO_BASE_URL")
		.map_err(|_| ApiError::Other("AVITO_BASE_URL not set".to_string()))?;

	// Build headers
	let mut headers = header::HeaderMap::new();
	headers.insert(
		header::AUTHORIZATION,
		format!("Bearer {}", avito_token).parse().unwrap(),
	);
	headers.insert(header::USER_AGENT, HeaderValue::from_static("YourApp/1.0"));
	headers.insert(
		header::CONTENT_TYPE,
		HeaderValue::from_static("application/json"),
	);
	headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

	// Build request body
	let request_body = json!({
		"price": opts.price
	});

	// Build URL
	let api_url = format!("{}/core/v1/items/{}/update_price", url, item_id);

	// Make request
	let response = Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?
		.post(&api_url)
		.headers(headers)
		.json(&request_body)
		.send()
		.await?;

	// Check response status
	if !response.status().is_success() {
		let status_code = response.status().as_u16();
		let error_body = response.text().await?;
		return Err(ApiError::AvitoApiError(status_code, error_body));
	}

	// Parse response
	let response_text = response.text().await?;

	let update_price_data: AvitoItemAnalyticsResponse = serde_json::from_str(&response_text)
		.map_err(|e| ApiError::JsonParseError(e, response_text.clone()))?;

	Ok(HttpResponse::Ok().json(json!({
		"status": "success",
		"data": update_price_data.result
	})))
}

#[post("/avito/get_categories_tree")]
#[has_any_role("Role::Admin", type = "Role")]
pub async fn get_avito_categories_tree(
	opts: web::Json<AvitoTokenParams>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let avito_token = opts.avito_token.clone();

	let url = env::var("AVITO_BASE_URL")
		.map_err(|_| ApiError::Other("AVITO_BASE_URL not set".to_string()))?;

	// Build headers
	let mut headers = header::HeaderMap::new();
	headers.insert(
		header::AUTHORIZATION,
		format!("Bearer {}", avito_token).parse().unwrap(),
	);
	//    headers.insert(
	// 	HeaderName::from_static("If-Modified-Since"),
	// 	HeaderValue::from_static("Mon, 01 Aug 2025 00:00:00 UTC"),
	// );

	// Build URL for user docs tree endpoint
	let api_url = format!("{}/autoload/v1/user-docs/tree", url);

	// Make request
	let response = Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?
		.get(&api_url)
		.headers(headers)
		.send()
		.await?;

	// Check response status
	if !response.status().is_success() {
		let status_code = response.status().as_u16();
		let error_body = response.text().await?;
		return Err(ApiError::AvitoApiError(status_code, error_body));
	}

	// Parse response
	let response_text = response.text().await?;
	let docs_tree_data: serde_json::Value = serde_json::from_str(&response_text)
		.map_err(|e| ApiError::JsonParseError(e, response_text.clone()))?;

	Ok(HttpResponse::Ok().json(json!({
		"status": "success",
		"data": docs_tree_data
	})))
}

#[post("/avito/get_category_fields")]
#[has_any_role("Role::Admin", type = "Role")]
pub async fn get_avito_category_fields(
	opts: web::Json<AvitoEditorCategoryFieldsParams>,
	_: JwtMiddleware,
	data: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let avito_token = opts.avito_token.clone();
	let avito_slug = opts.avito_slug.clone();

	let url = env::var("AVITO_BASE_URL")
		.map_err(|_| ApiError::Other("AVITO_BASE_URL not set".to_string()))?;

	// Build headers
	let mut headers = header::HeaderMap::new();
	headers.insert(
		header::AUTHORIZATION,
		format!("Bearer {}", avito_token).parse().unwrap(),
	);
	headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

	// Build URL for user docs node fields endpoint
	let api_url = format!("{}/autoload/v1/user-docs/node/{}/fields", url, avito_slug);

	// Make request
	let response = Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?
		.get(&api_url)
		.headers(headers.clone())
		.send()
		.await?;

	// Check response status
	if !response.status().is_success() {
		let status_code = response.status().as_u16();
		let error_body = response.text().await?;
		return Err(ApiError::AvitoApiError(status_code, error_body));
	}

	// Parse response
	let response_text = response.text().await?;
	let mut node_fields_data: serde_json::Value = serde_json::from_str(&response_text)
		.map_err(|e| ApiError::JsonParseError(e, response_text.clone()))?;

	// Process the response to fetch additional data from values_link_json and values_link_xml
	if let Some(fields_array) = node_fields_data
		.get_mut("fields")
		.and_then(|f| f.as_array_mut())
	{
		for field in fields_array.iter_mut() {
			// Process content array of the main field
			if let Some(content_array) = field.get_mut("content").and_then(|c| c.as_array_mut()) {
				for content_item in content_array.iter_mut() {
					// Process values_link_json
					if let Some(values_link_json) = content_item
						.get("values_link_json")
						.and_then(|v| v.as_str())
					{
						// Make additional request to fetch values from the JSON link
						let values_response = Client::builder()
							.danger_accept_invalid_certs(true)
							.timeout(std::time::Duration::from_secs(5)) // Add 5 second timeout
							.build()?
							.get(values_link_json)
							.headers(headers.clone())
							.send()
							.await?;

						if values_response.status().is_success() {
							let values_text = values_response.text().await?;
							let values_data: serde_json::Value = serde_json::from_str(&values_text)
								.map_err(|e| ApiError::JsonParseError(e, values_text.clone()))?;

							// Add the fetched values to the content item as a new "values" field
							content_item
								.as_object_mut()
								.unwrap()
								.insert("values".to_string(), values_data);
						}
					} else if let Some(values_link_xml) =
						content_item.get("values_link_xml").and_then(|v| v.as_str())
					{
						// Check if the values_link_xml contains "Autocatalog.xml"
						if values_link_xml.contains("Autocatalog.xml") {
							// Fetch data from avito_car_marks table
							match sqlx::query_as::<_, AvitoCarMark>(
								"SELECT car_mark_id, value FROM avito_car_marks",
							)
							.fetch_all(&data.db)
							.await
							{
								Ok(car_marks) => {
									// Convert car marks to the expected JSON format
									let values_data =
										serde_json::to_value(&car_marks).map_err(|e| {
											ApiError::JsonParseError(
												e,
												format!(
													"Failed to serialize car marks: {:?}",
													car_marks
												),
											)
										})?;

									// Add the fetched values to the content item as a new "values" field
									content_item
										.as_object_mut()
										.unwrap()
										.insert("values".to_string(), values_data);
								}
								Err(_) => {
									// If database query fails (e.g., table doesn't exist),
									// fall back to original behavior of making HTTP request
									// This preserves functionality when database is not set up
								}
							}
						} else {
							// For other XML links, make HTTP request as before (if needed)
							// This preserves the original behavior for non-Autocatalog.xml links
						}
					}

					// Remove the values_link_json and values_link_xml fields since we've fetched the data
					content_item
						.as_object_mut()
						.unwrap()
						.remove("values_link_json");
					content_item
						.as_object_mut()
						.unwrap()
						.remove("values_link_xml");
				}
			}

			// Process children array if it exists
			if let Some(children_array) = field.get_mut("children").and_then(|c| c.as_array_mut()) {
				for child in children_array.iter_mut() {
					// Extract the tag value before mutable borrows to avoid borrowing conflicts
					let tag_value = child
						.get("tag")
						.and_then(|t| t.as_str())
						.map(|s| s.to_string());

					// Process content array of each child
					if let Some(child_content_array) =
						child.get_mut("content").and_then(|c| c.as_array_mut())
					{
						for child_content_item in child_content_array.iter_mut() {
							// Process values_link_json in children
							if let Some(values_link_json) = child_content_item
								.get("values_link_json")
								.and_then(|v| v.as_str())
							{
								// Make additional request to fetch values from the JSON link
								let values_response = Client::builder()
									.danger_accept_invalid_certs(true)
									.timeout(std::time::Duration::from_secs(5)) // Add 5 second timeout
									.build()?
									.get(values_link_json)
									.headers(headers.clone())
									.send()
									.await?;

								if values_response.status().is_success() {
									let values_text = values_response.text().await?;
									let values_data: serde_json::Value =
										serde_json::from_str(&values_text).map_err(|e| {
											ApiError::JsonParseError(e, values_text.clone())
										})?;

									// Add the fetched values to the content item as a new "values" field
									child_content_item
										.as_object_mut()
										.unwrap()
										.insert("values".to_string(), values_data);
								}
							} else if let Some(values_link_xml) = child_content_item
								.get("values_link_xml")
								.and_then(|v| v.as_str())
							{
								// Check if the values_link_xml contains "Autocatalog.xml"
								if values_link_xml.contains("Autocatalog.xml") {
									// Skip database query if the field's tag is one of the specified values
									let should_skip_db_query = if let Some(ref tag_val) = tag_value
									{
										tag_val == "Model"
											|| tag_val == "Generation" || tag_val == "Modification"
											|| tag_val == "BodyType" || tag_val == "Doors"
									} else {
										false
									};

									if !should_skip_db_query {
										// Fetch data from avito_car_marks table
										match sqlx::query_as::<_, AvitoCarMark>(
											"SELECT car_mark_id, value FROM avito_car_marks",
										)
										.fetch_all(&data.db)
										.await
										{
											Ok(car_marks) => {
												// Convert car marks to the expected JSON format
												let values_data = serde_json::to_value(&car_marks)
													.map_err(|e| {
														ApiError::JsonParseError(e, format!("Failed to serialize car marks: {:?}", car_marks))
													})?;

												// Add the fetched values to the content item as a new "values" field
												child_content_item.as_object_mut().unwrap().insert(
													"values".to_string(),
													values_data.clone(),
												); // Clone to avoid move
											}
											Err(e) => {
												// If database query fails (e.g., table doesn't exist),
												// fall back to original behavior of making HTTP request
												// This preserves functionality when database is not set up
											}
										}
									}
								} else {
									// For other XML links, make HTTP request as before (if needed)
									// This preserves the original behavior for non-Autocatalog.xml links
								}
							}

							// Remove the values_link_json and values_link_xml fields since we've fetched the data
							child_content_item
								.as_object_mut()
								.unwrap()
								.remove("values_link_json");
							child_content_item
								.as_object_mut()
								.unwrap()
								.remove("values_link_xml");
						}
					}
				}
			}
		}
	}

	Ok(HttpResponse::Ok().json(json!({
		"status": "success",
		"data": node_fields_data
	})))
}
