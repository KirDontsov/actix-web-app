use crate::controllers::auth::Role;
use crate::{
	jwt_auth::JwtMiddleware,
	models::{ApiError, AvitoFeedAds},
	AppState,
};
use actix_web::{
	post,
	web::{self},
	HttpResponse,
};
use actix_web_grants::proc_macro::has_any_role;

use reqwest::Client;

use quick_xml::de::from_str;
use std::time::Duration;

// TODO: add import file form
// 1. By link
// 2. By import file

#[has_any_role("Role::Admin", type = "Role")]
#[post("/avito/import-xml")]
pub async fn import_avito_xml(
	data: web::Data<AppState>,
	_: JwtMiddleware,
) -> Result<HttpResponse, ApiError> {
	// Use provided URL or default
	let xml_url = "https://remzapchasti.ru/upload/avito_new_2.xml";
	let account_id = "2acc3808-15f1-4abb-b15e-c7f4780a87da";

	// Build HTTP client with timeout
	let client = Client::builder()
		.timeout(Duration::from_secs(30))
		.build()
		.map_err(|e| ApiError::InternalServerError(e.to_string()))?;

	// Fetch XML data
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

	// Parse XML
	let ads: AvitoFeedAds =
		from_str(&xml_data).map_err(|e| ApiError::Other(format!("Failed to parse XML: {}", e)))?;

	// Start database transaction
	let mut tx = data.db.begin().await.map_err(|e| {
		ApiError::InternalServerError(format!("Failed to start transaction: {}", e))
	})?;

	// Insert each ad
	for (index, ad) in ads.ads.iter().enumerate() {
		// Fix image URLs extraction
		let image_urls = ad
			.images
			.as_ref()
			.map(|img_struct| {
				img_struct
					.images
					.iter()
					.map(|img| img.url.as_str())
					.collect::<Vec<&str>>()
					.join(",\n")
			})
			.unwrap_or_default();

		// Parse delivery options
		let delivery_options = ad
			.delivery
			.as_ref()
			.map(|del| del.options.join(",\n"))
			.unwrap_or_default();

		println!("Processing ad {}: {}", index, ad.id);

		// Fixed SQL query - removed trailing comma and fixed parameter count
		let result = sqlx::query!(
			r#"
            INSERT INTO avito_ads (
                ad_id, account_id, goods_type, category, product_type, technic, spare_part_type,
                technic_spare_part_type, make, availability, ad_type, condition,
                originality, original_oem, oem, price, price_with_vat, image_urls,
                video_url, video_file_url, contact_phone, internet_calls, manager_name,
                brand, weight_for_delivery, height_for_delivery, width_for_delivery,
                length_for_delivery, delivery_options, multi_item, address, title, description
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33
            )
            ON CONFLICT (ad_id) DO UPDATE SET
                goods_type = EXCLUDED.goods_type,
                category = EXCLUDED.category,
                product_type = EXCLUDED.product_type,
                technic = EXCLUDED.technic,
                spare_part_type = EXCLUDED.spare_part_type,
                technic_spare_part_type = EXCLUDED.technic_spare_part_type,
                make = EXCLUDED.make,
                availability = EXCLUDED.availability,
                ad_type = EXCLUDED.ad_type,
                condition = EXCLUDED.condition,
                originality = EXCLUDED.originality,
                original_oem = EXCLUDED.original_oem,
                oem = EXCLUDED.oem,
                price = EXCLUDED.price,
                price_with_vat = EXCLUDED.price_with_vat,
                image_urls = EXCLUDED.image_urls,
                video_url = EXCLUDED.video_url,
                video_file_url = EXCLUDED.video_file_url,
                contact_phone = EXCLUDED.contact_phone,
                internet_calls = EXCLUDED.internet_calls,
                manager_name = EXCLUDED.manager_name,
                brand = EXCLUDED.brand,
                weight_for_delivery = EXCLUDED.weight_for_delivery,
                height_for_delivery = EXCLUDED.height_for_delivery,
                width_for_delivery = EXCLUDED.width_for_delivery,
                length_for_delivery = EXCLUDED.length_for_delivery,
                delivery_options = EXCLUDED.delivery_options,
                multi_item = EXCLUDED.multi_item,
                address = EXCLUDED.address,
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                updated_ts = CURRENT_TIMESTAMP
            "#,
			ad.id,
			account_id,
			ad.goods_type,
			ad.category,
			ad.product_type,
			ad.technic,
			ad.spare_part_type,
			ad.technic_spare_part_type,
			ad.make,
			ad.availability,
			ad.ad_type,
			ad.condition,
			ad.originality,
			ad.original_oem,
			ad.oem,
			ad.price,
			ad.price_with_vat,
			image_urls,
			ad.video_url,
			ad.video_file_url,
			ad.contact_phone,
			ad.internet_calls,
			ad.manager_name,
			ad.brand,
			ad.weight_for_delivery,
			ad.height_for_delivery,
			ad.width_for_delivery,
			ad.length_for_delivery,
			delivery_options,
			ad.multi_item,
			ad.address,
			ad.title,
			ad.description
		)
		.execute(&mut *tx)
		.await;

		match result {
			Ok(_) => println!("Successfully inserted ad {}", ad.id),
			Err(e) => {
				eprintln!("Failed to insert ad {}: {}", ad.id, e);
				// Continue with next ad instead of failing completely
				continue;
			}
		}
	}

	// Commit transaction
	tx.commit().await.map_err(|e| {
		ApiError::InternalServerError(format!("Failed to commit transaction: {}", e))
	})?;

	Ok(HttpResponse::Ok().json(serde_json::json!({
		"status": "success",
		"message": "Import completed. Check logs for any errors."
	})))
}
