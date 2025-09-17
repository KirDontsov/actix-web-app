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

use quick_xml::events::Event;
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
	_: JwtMiddleware,
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
	let ads = parse_xml_ads(&xml_data)?;

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
	batch_process_ads(&mut tx, &feed_id, &ads).await?;

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
	let mut avito_ids = Vec::new();
	let mut parsed_ids = Vec::new();
	let mut feed_ids = Vec::new();
	let mut is_active_flags = Vec::new();
	let mut statuses = Vec::new();

	// Prepare batch data for ads
	for ad in ads {
		let ad_id = Uuid::new_v4();
		ad_ids.push(ad_id);
		avito_ids.push(ad.id.clone()); // Clone the String
		parsed_ids.push(ad.id.clone()); // Clone the String
		feed_ids.push(feed_id.clone()); // Clone the Uuid
		is_active_flags.push(true);
		statuses.push("active".to_string());
	}

	// Batch insert ads
	sqlx::query!(
		r#"
        INSERT INTO avito_ads (ad_id, avito_ad_id, parsed_id, feed_id, is_active, status)
        SELECT * FROM UNNEST(
            $1::uuid[],
            $2::varchar[],
            $3::varchar[],
            $4::uuid[],
            $5::bool[],
            $6::varchar[]
        )
        "#,
		&ad_ids,
		&avito_ids,
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

fn parse_xml_ads(xml_data: &str) -> Result<Vec<XmlAd>, ApiError> {
	let mut reader = Reader::from_str(xml_data);
	let mut ads = Vec::new();
	let mut buf = Vec::new();
	let mut current_ad: Option<XmlAd> = None;
	let mut current_path = Vec::new();
	let mut current_value = String::new();
	let mut in_ad = false;

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(e)) => {
				let name = std::str::from_utf8(e.name().as_ref())
					.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
					.to_string();

				current_path.push(name.clone());

				if name == "Ad" {
					in_ad = true;
					current_ad = Some(XmlAd {
						id: String::new(),
						fields: HashMap::new(),
					});
				}

				current_value.clear();
			}
			Ok(Event::Text(e)) => {
				// Extract text content directly from the bytes
				let text = std::str::from_utf8(e.into_inner().as_ref())
					.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
					.to_string();

				if in_ad {
					current_value.push_str(&text);
				}
			}
			Ok(Event::End(e)) => {
				let name = std::str::from_utf8(e.name().as_ref())
					.map_err(|e| ApiError::Other(format!("UTF-8 error: {}", e)))?
					.to_string();

				if let Some(ad) = &mut current_ad {
					// Store the field value if it's not empty
					if !current_value.trim().is_empty() && current_path.len() > 1 {
						let field_name = current_path.last().unwrap().clone();
						ad.fields
							.insert(field_name, current_value.trim().to_string());
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

				current_path.pop();
				current_value.clear();
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

#[get("/avito/ads_by_feed")]
pub async fn get_avito_feeds(
	// opts: web::Query<FeedQueryParams>,
	data: web::Data<AppState>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	// let page = opts.page.unwrap_or(1);
	// let limit = opts.limit.unwrap_or(10);
	// let offset = (page - 1) * limit;

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
      ORDER BY f.created_ts DESC, a.created_ts DESC, af.created_ts DESC;
    "#,
	)
	// .bind(limit as i64)
	// .bind(offset as i64)
	.fetch_all(&data.db)
	.await
	.map_err(|e| ApiError::InternalServerError(format!("Failed to fetch feeds: {}", e)))?;

	// Transform rows into our structs
	let mut feeds: Vec<FeedResponse> = Vec::new();
	let mut current_feed_id: Option<Uuid> = None;
	let mut current_ad_id: Option<Uuid> = None;
	let mut current_field_id: Option<Uuid> = None;

	for row in rows {
		// Extract values from row with proper error handling
		let feed_id: Uuid = match row.try_get("feed_id") {
			Ok(id) => id,
			Err(e) => {
				println!("Error getting feed_id: {}", e);
				return Err(ApiError::InternalServerError(format!(
					"Failed to get feed_id: {}",
					e
				)));
			}
		};

		let account_id: Uuid = match row.try_get("account_id") {
			Ok(id) => id,
			Err(e) => {
				println!("Error getting account_id: {}", e);
				return Err(ApiError::InternalServerError(format!(
					"Failed to get account_id: {}",
					e
				)));
			}
		};

		let category: String = match row.try_get("category") {
			Ok(cat) => cat,
			Err(e) => {
				println!("Error getting category: {}", e);
				return Err(ApiError::InternalServerError(format!(
					"Failed to get category: {}",
					e
				)));
			}
		};

		let feed_created_ts: chrono::DateTime<chrono::Utc> = match row.try_get("feed_created_ts") {
			Ok(ts) => ts,
			Err(e) => {
				println!("Error getting feed_created_ts: {}", e);
				return Err(ApiError::InternalServerError(format!(
					"Failed to get feed_created_ts: {}",
					e
				)));
			}
		};

		// Handle feed level
		if current_feed_id != Some(feed_id) {
			feeds.push(FeedResponse {
				feed_id,
				account_id,
				category,
				created_ts: feed_created_ts,
				ads: Vec::new(),
			});
			current_feed_id = Some(feed_id);
		}

		// Handle ad level (might be NULL due to LEFT JOIN)
		let ad_id: Option<Uuid> = row.try_get("ad_id").ok();
		let avito_ad_id: Option<String> = row.try_get("avito_ad_id").ok();
		let parsed_id: Option<String> = row.try_get("parsed_id").ok();
		let is_active: Option<bool> = row.try_get("is_active").ok();
		let status: Option<String> = row.try_get("status").ok();
		let ad_created_ts: Option<chrono::DateTime<chrono::Utc>> =
			row.try_get("ad_created_ts").ok();

		if let (
			Some(ad_id),
			Some(avito_ad_id),
			Some(parsed_id),
			Some(is_active),
			Some(status),
			Some(ad_created_ts),
		) = (
			ad_id,
			avito_ad_id,
			parsed_id,
			is_active,
			status,
			ad_created_ts,
		) {
			if current_ad_id != Some(ad_id) {
				if let Some(feed) = feeds.last_mut() {
					feed.ads.push(AdResponse {
						ad_id,
						avito_ad_id,
						parsed_id,
						is_active,
						status,
						created_ts: ad_created_ts,
						fields: Vec::new(),
					});
					current_ad_id = Some(ad_id);
				}
			}

			// Handle field level (might be NULL due to LEFT JOIN)
			let field_id: Option<Uuid> = row.try_get("field_id").ok();
			let tag: Option<String> = row.try_get("tag").ok();
			let data_type: Option<String> = row.try_get("data_type").ok();
			let field_type: Option<String> = row.try_get("field_type").ok();
			let field_created_ts: Option<chrono::DateTime<chrono::Utc>> =
				row.try_get("field_created_ts").ok();

			if let (
				Some(field_id),
				Some(tag),
				Some(data_type),
				Some(field_type),
				Some(field_created_ts),
			) = (field_id, tag, data_type, field_type, field_created_ts)
			{
				if current_field_id != Some(field_id) {
					if let Some(feed) = feeds.last_mut() {
						if let Some(ad) = feed.ads.last_mut() {
							ad.fields.push(FieldResponse {
								field_id,
								tag,
								data_type,
								field_type,
								created_ts: field_created_ts,
								values: Vec::new(),
							});
							current_field_id = Some(field_id);
						}
					}
				}

				// Handle field value level (might be NULL due to LEFT JOIN)
				let field_value_id: Option<Uuid> = row.try_get("field_value_id").ok();
				let value: Option<String> = row.try_get("value").ok();
				let value_created_ts: Option<chrono::DateTime<chrono::Utc>> =
					row.try_get("value_created_ts").ok();

				if let (Some(field_value_id), Some(value), Some(value_created_ts)) =
					(field_value_id, value, value_created_ts)
				{
					if let Some(feed) = feeds.last_mut() {
						if let Some(ad) = feed.ads.last_mut() {
							if let Some(field) = ad.fields.last_mut() {
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
		}
	}

	Ok(HttpResponse::Ok().json(serde_json::json!({
		"status": "success",
		"data": feeds
	})))
}
