use crate::controllers::auth::Role;
use crate::{
	jwt_auth::JwtMiddleware,
	models::{
		AdResponse, ApiError, AvitoFeedAds, FeedJoinRow, FeedQueryParams, FeedResponse,
		FieldResponse, FieldValueResponse, XmlAd,
	},
	AppState,
};
use actix_web::{
	get, post,
	web::{self},
	HttpResponse,
};
use actix_web_grants::proc_macro::has_any_role;

use quick_xml::events::{attributes::Attribute, Event};
use quick_xml::Reader;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::Row;
use sqlx::{Postgres, Transaction};
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;
use uuid::Uuid;

// TODO: add import file form
// 1. By link
// 2. By import file

#[post("/avito/import-xml")]
pub async fn import_avito_xml(
	data: web::Data<AppState>,
	// _: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let xml_url = "https://remzapchasti.ru/upload/avito_new_2.xml";
	let account_id = Uuid::parse_str("2acc3808-15f1-4abb-b15e-c7f4780a87da").unwrap();

	// Fetch XML data
	let client = Client::builder()
		.timeout(Duration::from_secs(30))
		.build()
		.map_err(|e| ApiError::InternalServerError(e.to_string()))?;

	let response = client
		.get(xml_url)
		.send()
		.await
		.map_err(|e| ApiError::InternalServerError(format!("Failed to fetch XML: {}", e)))?;

	if !response.status().is_success() {
		return Err(ApiError::InternalServerError(format!(
			"Failed to fetch XML: Status {}",
			response.status()
		)));
	}

	let xml_data = response
		.text()
		.await
		.map_err(|e| ApiError::InternalServerError(format!("Failed to read response: {}", e)))?;

	// Parse XML and extract ads
	println!("Parsing XML data with length: {}", xml_data.len());
	let ads = parse_xml_ads(&xml_data)?;
	println!("Parsed {} ads from XML", ads.len());

	// Print debug information about the first few ads
	for (i, ad) in ads.iter().take(3).enumerate() {
		println!("Ad {}: id={}, fields={}", i, ad.id, ad.fields.len());
		if let Some(images) = ad.fields.get("Images") {
			println!("  Images field: {}", images);
		} else {
			println!("  No Images field found");
		}
	}

	println!("Parsed {} ads from XML", ads.len());

	// Start transaction
	let mut tx = data.db.begin().await.map_err(|e| {
		ApiError::InternalServerError(format!("Failed to start transaction: {}", e))
	})?;

	// Create feed entry
	let feed_id = Uuid::new_v4();
	sqlx::query!(
		r#"
        INSERT INTO avito_feeds (feed_id, account_id, category)
        VALUES ($1, $2, $3)
        "#,
		feed_id,
		account_id,
		"IMPORT"
	)
	.execute(&mut *tx)
	.await
	.map_err(|e| ApiError::InternalServerError(format!("Failed to create feed: {}", e)))?;

	// Batch process ads
	println!("Starting batch processing of {} ads", ads.len());
	batch_process_ads(&mut tx, &feed_id, &ads).await?;
	println!("Finished batch processing");

	tx.commit().await.map_err(|e| {
		ApiError::InternalServerError(format!("Failed to commit transaction: {}", e))
	})?;

	Ok(HttpResponse::Ok().json(serde_json::json!({
		"status": "success",
		"message": "Import completed successfully",
		"feed_id": feed_id,
		"ads_processed": ads.len()
	})))
}

// Batch process ads for better performance
async fn batch_process_ads(
	tx: &mut Transaction<'_, Postgres>,
	feed_id: &Uuid,
	ads: &[XmlAd],
) -> Result<(), ApiError> {
	let mut ad_ids = Vec::new();
	let mut parsed_ids = Vec::new();
	let mut feed_ids = Vec::new();
	let mut is_active_flags = Vec::new();
	let mut statuses = Vec::new();

	// Prepare batch data for ads
	for ad in ads {
		let ad_id = Uuid::new_v4();
		ad_ids.push(ad_id);
		parsed_ids.push(ad.id.clone()); // Clone the String
		feed_ids.push(feed_id.clone()); // Clone the Uuid
		is_active_flags.push(true);
		statuses.push("active".to_string());
	}

	// Batch insert ads
	sqlx::query!(
		r#"
        INSERT INTO avito_ads (ad_id, parsed_id, feed_id, is_active, status)
        SELECT * FROM UNNEST(
            $1::uuid[],
            $2::varchar[],
            $3::uuid[],
            $4::bool[],
            $5::varchar[]
        )
        "#,
		&ad_ids,
		&parsed_ids,
		&feed_ids,
		&is_active_flags,
		&statuses,
	)
	.execute(&mut **tx)
	.await
	.map_err(|e| ApiError::InternalServerError(format!("Failed to batch insert ads: {}", e)))?;

	println!("Inserted {} ads in batch", ads.len());

	// Batch process fields
	let mut field_ids = Vec::new();
	let mut field_ad_ids = Vec::new();
	let mut field_tags = Vec::new();
	let mut field_data_types = Vec::new();
	let mut field_field_types = Vec::new();

	let mut field_value_ids = Vec::new();
	let mut field_value_field_ids = Vec::new();
	let mut field_values = Vec::new();

	for (i, ad) in ads.iter().enumerate() {
		let ad_id = ad_ids[i];

		for (tag, value) in &ad.fields {
			// Skip Id field and other empty fields, but allow empty Images field
			if tag == "Id" || value.trim().is_empty() {
				continue;
			}

			let field_id = Uuid::new_v4();
			field_ids.push(field_id);
			field_ad_ids.push(ad_id);
			field_tags.push(tag.clone()); // Clone the String
			field_data_types.push("string".to_string());
			field_field_types.push("attribute".to_string());

			let field_value_id = Uuid::new_v4();
			field_value_ids.push(field_value_id);
			field_value_field_ids.push(field_id);
			field_values.push(value.clone()); // Clone the String
		}
	}

	// Batch insert fields
	if !field_ids.is_empty() {
		sqlx::query!(
			r#"
            INSERT INTO avito_ad_fields (field_id, ad_id, tag, data_type, field_type)
            SELECT * FROM UNNEST(
                $1::uuid[],
                $2::uuid[],
                $3::varchar[],
                $4::varchar[],
                $5::varchar[]
            )
            "#,
			&field_ids,
			&field_ad_ids,
			&field_tags,
			&field_data_types,
			&field_field_types,
		)
		.execute(&mut **tx)
		.await
		.map_err(|e| {
			ApiError::InternalServerError(format!("Failed to batch insert fields: {}", e))
		})?;

		println!("Inserted {} fields in batch", field_ids.len());
	}

	// Batch insert field values
	if !field_value_ids.is_empty() {
		sqlx::query!(
			r#"
            INSERT INTO avito_ad_field_values (field_value_id, field_id, value)
            SELECT * FROM UNNEST(
                $1::uuid[],
                $2::uuid[],
                $3::varchar[]
            )
            "#,
			&field_value_ids,
			&field_value_field_ids,
			&field_values,
		)
		.execute(&mut **tx)
		.await
		.map_err(|e| {
			ApiError::InternalServerError(format!("Failed to batch insert field values: {}", e))
		})?;

		println!("Inserted {} field values in batch", field_value_ids.len());
	}

	Ok(())
}

pub fn parse_xml_ads(xml_data: &str) -> Result<Vec<XmlAd>, ApiError> {
	let mut reader = Reader::from_str(xml_data);
	let mut ads = Vec::new();
	let mut buf = Vec::new();
	let mut current_ad: Option<XmlAd> = None;
	let mut current_path = Vec::new();
	let mut current_values = String::new();
	let mut in_ad = false;
	let mut delivery_buffer = Vec::new();
	let mut images_buffer: Vec<String> = Vec::new();

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(e)) => {
				let name = std::str::from_utf8(e.name().as_ref())
					.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
					.to_string();

				// println!("Start element: {}, current path: {:?}", name, current_path);
				current_path.push(name.clone());

				if name == "Ad" {
					in_ad = true;
					current_ad = Some(XmlAd {
						id: String::new(),
						fields: HashMap::new(),
					});
					delivery_buffer.clear();
					images_buffer.clear();
				}

				current_values.clear();
			}
			Ok(Event::Text(e)) => {
				// Extract text content directly from the bytes
				let text = std::str::from_utf8(e.into_inner().as_ref())
					.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
					.to_string();

				if in_ad && !&text.trim().is_empty() {
					current_values.push_str(&text);
				}
			}
			Ok(Event::CData(e)) => {
				// Handle CDATA content (for Description)
				let text = std::str::from_utf8(e.as_ref())
					.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
					.to_string();

				if in_ad {
					current_values.push_str(&text);
				}
			}
			Ok(Event::Empty(e)) => {
				let name = std::str::from_utf8(e.name().as_ref())
					.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
					.to_string();

				// Handle Image tags with attributes
				if name == "Image" && current_path.contains(&"Images".to_string()) {
					// Extract the url attribute directly and add to images_buffer
					for attr_result in e.attributes() {
						if let Ok(attr) = attr_result {
							if attr.key.as_ref() == b"url" {
								if let Ok(url) = std::str::from_utf8(&attr.value) {
									images_buffer.push(url.to_string());
								}
							}
						}
					}
				}
				// Handle other empty elements (fallback)
				else {
					let text = std::str::from_utf8(e.as_ref())
						.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
						.to_string();

					if in_ad {
						current_values.push_str(&text);
					}
				}
			}
			Ok(Event::End(e)) => {
				let name = std::str::from_utf8(e.name().as_ref())
					.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
					.to_string();

				if let Some(ad) = &mut current_ad {
					// Special handling for Delivery - store as comma-separated options
					if name == "Delivery" && !delivery_buffer.is_empty() {
						ad.fields
							.insert("Delivery".to_string(), delivery_buffer.join(","));
						delivery_buffer.clear();
					}
					// Special handling for Option elements inside Delivery
					else if name == "Option" && current_path.contains(&"Delivery".to_string()) {
						if !current_values.trim().is_empty() {
							delivery_buffer.push(current_values.trim().to_string());
						}
					} else if name == "Images" {
						// Store image URLs when closing Images tag
						if !images_buffer.is_empty() {
							ad.fields
								.insert("Images".to_string(), images_buffer.join(","));
							images_buffer.clear();
						}
					} else if name == "Image" && current_path.contains(&"Images".to_string()) {
						// For non-empty Image tags, add their text content to images_buffer
						if !current_values.trim().is_empty() {
							images_buffer.push(current_values.trim().to_string());
						}
					}
					// Store other field values if not empty
					else if !current_values.trim().is_empty() && current_path.len() > 1 {
						let field_name = current_path.last().unwrap().clone();
						// Skip storing individual Image and Option elements as they're handled specially
						if field_name != "Image" && field_name != "Option" {
							ad.fields
								.insert(field_name, current_values.trim().to_string());
						}
					}

					// Special handling for Id field
					if name == "Id" {
						if let Some(id_value) = ad.fields.get("Id") {
							ad.id = id_value.clone();
						}
					}
				}

				// If this is the end of an Ad element, add it to the list
				if name == "Ad" {
					in_ad = false;
					if let Some(ad) = current_ad.take() {
						if !ad.id.is_empty() {
							ads.push(ad);
						}
					}
				}

				// println!("End element: {}, current path: {:?}", name, current_path);
				current_path.pop();
				current_values.clear();
			}
			Ok(Event::Eof) => break,
			Err(e) => return Err(ApiError::Other(format!("XML parse error: {}", e))),
			_ => (),
		}

		// Clear the buffer to prevent re-processing the same event
		buf.clear();
	}

	println!("Finished parsing {} ads", ads.len());
	Ok(ads)
}

#[get("/avito/get_feeds")]
pub async fn get_avito_feeds(
	opts: web::Query<FeedQueryParams>,
	data: web::Data<AppState>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let page = opts.page.unwrap_or(1);
	let limit = opts.limit.unwrap_or(10);
	let offset = (page - 1) * limit;

	// Fetch data with manual row mapping
	let rows = sqlx::query(
		r#"
	     SELECT
	       f.feed_id,
	       f.account_id,
	       f.category,
	       f.created_ts as feed_created_ts,
	       a.ad_id,
	       a.avito_ad_id,
	       a.parsed_id,
	       a.is_active,
	       a.status,
	       a.created_ts as ad_created_ts,
	       af.field_id,
	       af.tag,
	       af.data_type,
	       af.field_type,
	       af.created_ts as field_created_ts,
	       afv.field_value_id,
	       afv.value,
	       afv.created_ts as value_created_ts
	     FROM avito_feeds f
	     LEFT JOIN avito_ads a ON f.feed_id = a.feed_id
	     LEFT JOIN avito_ad_fields af ON a.ad_id = af.ad_id
	     LEFT JOIN avito_ad_field_values afv ON af.field_id = afv.field_id
	     ORDER BY f.created_ts DESC, a.created_ts DESC, af.created_ts DESC
	     LIMIT $1 OFFSET $2;
	   "#,
	)
	.bind(limit as i64)
	.bind(offset as i64)
	.fetch_all(&data.db)
	.await
	.map_err(|e| ApiError::InternalServerError(format!("Failed to fetch feeds: {}", e)))?;

	// Fetch total count of feeds
	let count_row = sqlx::query!(
		r#"SELECT COUNT(*) as count FROM avito_feeds"#,
	)
	.fetch_one(&data.db)
	.await
	.map_err(|e| ApiError::InternalServerError(format!("Failed to fetch feed count: {}", e)))?;

	let total_feeds = count_row.count.unwrap_or(0) as u32;

	// Transform rows into our structs using proper grouping
	let mut feeds: std::collections::HashMap<Uuid, FeedResponse> = std::collections::HashMap::new();
	let mut ads: std::collections::HashMap<Uuid, AdResponse> = std::collections::HashMap::new();
	let mut fields: std::collections::HashMap<Uuid, FieldResponse> = std::collections::HashMap::new();

	for row in &rows {
		// Extract feed information
		let feed_id: Uuid = row.try_get("feed_id")
			.map_err(|e| ApiError::InternalServerError(format!("Failed to get feed_id: {}", e)))?;
		let account_id: Uuid = row.try_get("account_id")
			.map_err(|e| ApiError::InternalServerError(format!("Failed to get account_id: {}", e)))?;
		let category: String = row.try_get("category")
			.map_err(|e| ApiError::InternalServerError(format!("Failed to get category: {}", e)))?;
		let feed_created_ts: chrono::DateTime<chrono::Utc> = row.try_get("feed_created_ts")
			.map_err(|e| ApiError::InternalServerError(format!("Failed to get feed_created_ts: {}", e)))?;

		// Create or get feed
		feeds.entry(feed_id).or_insert_with(|| FeedResponse {
			feed_id,
			account_id,
			category,
			created_ts: feed_created_ts,
			ads: Vec::new(),
		});

		// Handle ad level (might be NULL due to LEFT JOIN)
		if let Ok(ad_id) = row.try_get::<Uuid, _>("ad_id") {
			let avito_ad_id: String = row.try_get("avito_ad_id")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get avito_ad_id: {}", e)))?;
			let parsed_id: String = row.try_get("parsed_id")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get parsed_id: {}", e)))?;
			let is_active: bool = row.try_get("is_active")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get is_active: {}", e)))?;
			let status: String = row.try_get("status")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get status: {}", e)))?;
			let ad_created_ts: chrono::DateTime<chrono::Utc> = row.try_get("ad_created_ts")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get ad_created_ts: {}", e)))?;

			// Create or get ad
			ads.entry(ad_id).or_insert_with(|| AdResponse {
				ad_id,
				avito_ad_id,
				parsed_id,
				is_active,
				status,
				created_ts: ad_created_ts,
				fields: Vec::new(),
			});

			// Handle field level (might be NULL due to LEFT JOIN)
			if let Ok(field_id) = row.try_get::<Uuid, _>("field_id") {
				let tag: String = row.try_get("tag")
					.map_err(|e| ApiError::InternalServerError(format!("Failed to get tag: {}", e)))?;
				let data_type: String = row.try_get("data_type")
					.map_err(|e| ApiError::InternalServerError(format!("Failed to get data_type: {}", e)))?;
				let field_type: String = row.try_get("field_type")
					.map_err(|e| ApiError::InternalServerError(format!("Failed to get field_type: {}", e)))?;
				let field_created_ts: chrono::DateTime<chrono::Utc> = row.try_get("field_created_ts")
					.map_err(|e| ApiError::InternalServerError(format!("Failed to get field_created_ts: {}", e)))?;

				// Create or get field
				fields.entry(field_id).or_insert_with(|| FieldResponse {
					field_id,
					tag,
					data_type,
					field_type,
					created_ts: field_created_ts,
					values: Vec::new(),
				});

				// Handle field value level (might be NULL due to LEFT JOIN)
				if let Ok(field_value_id) = row.try_get::<Uuid, _>("field_value_id") {
					let value: String = row.try_get("value")
						.map_err(|e| ApiError::InternalServerError(format!("Failed to get value: {}", e)))?;
					let value_created_ts: chrono::DateTime<chrono::Utc> = row.try_get("value_created_ts")
						.map_err(|e| ApiError::InternalServerError(format!("Failed to get value_created_ts: {}", e)))?;

					// Add field value to field
					if let Some(field) = fields.get_mut(&field_id) {
						field.values.push(FieldValueResponse {
							field_value_id,
							value,
							created_ts: value_created_ts,
						});
					}
				}
			}
		}
	}

	// Build the final hierarchy: fields -> ads -> feeds
	// First, attach fields to their respective ads
	for (_, mut field) in fields {
		// Find the ad this field belongs to (we need to look at the rows again for this)
		for row in &rows {
			if row.try_get::<Uuid, _>("field_id").ok() == Some(field.field_id) {
				if let Ok(ad_id) = row.try_get::<Uuid, _>("ad_id") {
					if let Some(ad) = ads.get_mut(&ad_id) {
						ad.fields.push(field);
						break;
					}
				}
			}
		}
	}

	// Then, attach ads to their respective feeds
	for (_, mut ad) in ads {
		// Find the feed this ad belongs to (we need to look at the rows again for this)
		for row in &rows {
			if row.try_get::<Uuid, _>("ad_id").ok() == Some(ad.ad_id) {
				if let Ok(feed_id) = row.try_get::<Uuid, _>("feed_id") {
					if let Some(feed) = feeds.get_mut(&feed_id) {
						// Sort fields by created_ts for consistent ordering
						ad.fields.sort_by(|a, b| a.created_ts.cmp(&b.created_ts));
						feed.ads.push(ad);
						break;
					}
				}
			}
		}
	}

	// Convert HashMap to Vec and sort by created_ts for consistent ordering
	let mut feeds_vec: Vec<FeedResponse> = feeds.into_values().collect();
	feeds_vec.sort_by(|a, b| a.created_ts.cmp(&b.created_ts));

	// Sort ads within each feed by created_ts for consistent ordering
	for feed in &mut feeds_vec {
		feed.ads.sort_by(|a, b| a.created_ts.cmp(&b.created_ts));
	}

	Ok(HttpResponse::Ok().json(serde_json::json!({
		"status": "success",
		"data": feeds_vec,
		"pagination": {
			"page": page,
			"limit": limit,
			"total": total_feeds,
			"pages": (total_feeds as f64 / limit as f64).ceil() as u32
		}
	})))
}

#[get("/avito/get_feed")]
pub async fn get_last_avito_feed(
	opts: web::Query<FeedQueryParams>,
	data: web::Data<AppState>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	let page = opts.page.unwrap_or(1);
	let limit = opts.limit.unwrap_or(10);
	let offset = (page - 1) * limit;
	
	// First, get the most recent feed
	let latest_feed_row = sqlx::query!(
		r#"SELECT feed_id, account_id, category, created_ts 
		   FROM avito_feeds 
		   ORDER BY created_ts DESC 
		   LIMIT 1"#
	)
	.fetch_optional(&data.db)
	.await
	.map_err(|e| ApiError::InternalServerError(format!("Failed to fetch latest feed: {}", e)))?;
	
	// If no feed exists, return empty response
	let latest_feed = match latest_feed_row {
		Some(feed) => feed,
		None => {
			return Ok(HttpResponse::Ok().json(serde_json::json!({
				"status": "success",
				"data": null,
				"pagination": {
					"page": page,
					"limit": limit,
					"total": 0,
					"pages": 0
				}
			})));
		}
	};
	
	// First, get the paginated list of ad IDs for this feed
	let ad_ids_rows = sqlx::query!(
		r#"SELECT ad_id FROM avito_ads
		   WHERE feed_id = $1
		   ORDER BY created_ts DESC
		   LIMIT $2 OFFSET $3"#,
		latest_feed.feed_id,
		limit as i64,
		offset as i64
	)
	.fetch_all(&data.db)
	.await
	.map_err(|e| ApiError::InternalServerError(format!("Failed to fetch ad IDs: {}", e)))?;
	
	// Extract ad IDs into a vector
	let ad_ids: Vec<Uuid> = ad_ids_rows.into_iter().map(|row| row.ad_id).collect();
	
	// If we have ad IDs, get all the data for these ads
	let rows = if !ad_ids.is_empty() {
		// Convert Uuid vector to a format we can use in the query
		// For simplicity, we'll use a separate query for each ad ID and combine results
		let mut all_rows = Vec::new();
		
		for ad_id in &ad_ids {
			let ad_rows = sqlx::query(
				r#"SELECT
					a.ad_id,
					a.avito_ad_id,
					a.parsed_id,
					a.is_active,
					a.status,
					a.created_ts as ad_created_ts,
					af.field_id,
					af.tag,
					af.data_type,
					af.field_type,
					af.created_ts as field_created_ts,
					afv.field_value_id,
					afv.value,
					afv.created_ts as value_created_ts
				FROM avito_ads a
				LEFT JOIN avito_ad_fields af ON a.ad_id = af.ad_id
				LEFT JOIN avito_ad_field_values afv ON af.field_id = afv.field_id
				WHERE a.ad_id = $1
				ORDER BY a.created_ts DESC, af.created_ts DESC"#
			)
			.bind(ad_id)
			.fetch_all(&data.db)
			.await
			.map_err(|e| ApiError::InternalServerError(format!("Failed to fetch ad data: {}", e)))?;
			
			all_rows.extend(ad_rows);
		}
		
		all_rows
	} else {
		Vec::new()
	};
	
	// Get total count of ads for this feed
	let count_row = sqlx::query!(
		r#"SELECT COUNT(*) as count FROM avito_ads WHERE feed_id = $1"#,
		latest_feed.feed_id
	)
	.fetch_one(&data.db)
	.await
	.map_err(|e| ApiError::InternalServerError(format!("Failed to fetch ad count: {}", e)))?;
	
	let total_ads = count_row.count.unwrap_or(0) as u32;
	
	// Transform rows into our structs using proper grouping
	let mut ads: std::collections::HashMap<Uuid, AdResponse> = std::collections::HashMap::new();
	let mut fields: std::collections::HashMap<Uuid, FieldResponse> = std::collections::HashMap::new();
	
	for row in &rows {
		// Handle ad level (might be NULL due to LEFT JOIN)
		if let Ok(ad_id) = row.try_get::<Uuid, _>("ad_id") {
			let avito_ad_id: String = row.try_get("avito_ad_id")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get avito_ad_id: {}", e)))?;
			let parsed_id: String = row.try_get("parsed_id")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get parsed_id: {}", e)))?;
			let is_active: bool = row.try_get("is_active")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get is_active: {}", e)))?;
			let status: String = row.try_get("status")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get status: {}", e)))?;
			let ad_created_ts: chrono::DateTime<chrono::Utc> = row.try_get("ad_created_ts")
				.map_err(|e| ApiError::InternalServerError(format!("Failed to get ad_created_ts: {}", e)))?;
			
			// Create or get ad
			ads.entry(ad_id).or_insert_with(|| AdResponse {
				ad_id,
				avito_ad_id,
				parsed_id,
				is_active,
				status,
				created_ts: ad_created_ts,
				fields: Vec::new(),
			});
			
			// Handle field level (might be NULL due to LEFT JOIN)
			if let Ok(field_id) = row.try_get::<Uuid, _>("field_id") {
				let tag: String = row.try_get("tag")
					.map_err(|e| ApiError::InternalServerError(format!("Failed to get tag: {}", e)))?;
				let data_type: String = row.try_get("data_type")
					.map_err(|e| ApiError::InternalServerError(format!("Failed to get data_type: {}", e)))?;
				let field_type: String = row.try_get("field_type")
					.map_err(|e| ApiError::InternalServerError(format!("Failed to get field_type: {}", e)))?;
				let field_created_ts: chrono::DateTime<chrono::Utc> = row.try_get("field_created_ts")
					.map_err(|e| ApiError::InternalServerError(format!("Failed to get field_created_ts: {}", e)))?;
				
				// Create or get field
				fields.entry(field_id).or_insert_with(|| FieldResponse {
					field_id,
					tag,
					data_type,
					field_type,
					created_ts: field_created_ts,
					values: Vec::new(),
				});
				
				// Handle field value level (might be NULL due to LEFT JOIN)
				if let Ok(field_value_id) = row.try_get::<Uuid, _>("field_value_id") {
					let value: String = row.try_get("value")
						.map_err(|e| ApiError::InternalServerError(format!("Failed to get value: {}", e)))?;
					let value_created_ts: chrono::DateTime<chrono::Utc> = row.try_get("value_created_ts")
						.map_err(|e| ApiError::InternalServerError(format!("Failed to get value_created_ts: {}", e)))?;
					
					// Add field value to field
					if let Some(field) = fields.get_mut(&field_id) {
						field.values.push(FieldValueResponse {
							field_value_id,
							value,
							created_ts: value_created_ts,
						});
					}
				}
			}
		}
	}
	
	// Build the final hierarchy: fields -> ads
	// First, attach fields to their respective ads
	for (_, mut field) in fields {
		// Find the ad this field belongs to
		for row in &rows {
			if row.try_get::<Uuid, _>("field_id").ok() == Some(field.field_id) {
				if let Ok(ad_id) = row.try_get::<Uuid, _>("ad_id") {
					if let Some(ad) = ads.get_mut(&ad_id) {
						ad.fields.push(field);
						break;
					}
				}
			}
		}
	}
	
	// Convert HashMap to Vec and sort by created_ts for consistent ordering
	let mut ads_vec: Vec<AdResponse> = ads.into_values().collect();
	ads_vec.sort_by(|a, b| a.created_ts.cmp(&b.created_ts));
	
	// Sort fields within each ad by created_ts for consistent ordering
	for ad in &mut ads_vec {
		ad.fields.sort_by(|a, b| a.created_ts.cmp(&b.created_ts));
	}

	// Create the FeedResponse with the latest feed data and paginated ads
	let feed_response = FeedResponse {
		feed_id: latest_feed.feed_id,
		account_id: latest_feed.account_id,
		category: latest_feed.category.expect("No category"),
		created_ts: latest_feed.created_ts.expect("No created_ts"),
		ads: ads_vec,
	};
	
	Ok(HttpResponse::Ok().json(serde_json::json!({
		"status": "success",
		"data": feed_response,
		"pagination": {
			"page": page,
			"limit": limit,
			"total": total_ads,
			"pages": (total_ads as f64 / limit as f64).ceil() as u32
		}
	})))
}
