use crate::{
	models::{Count, Firm},
	utils::Translit,
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};

#[allow(unreachable_code)]
#[get("/processing/urls")]
async fn urls_processing_handler(
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	println!("start");
	let _: Result<(), Box<dyn std::error::Error>> = match processing(data.clone()).await {
		Ok(x) => Ok(x),
		Err(e) => {
			println!("{:?}", e);
			Err(e)
		}
	};

	let json_response = serde_json::json!({
		"status":  "success",
	});
	HttpResponse::Ok().json(json_response)
}

async fn processing(data: web::Data<AppState>) -> Result<(), Box<dyn std::error::Error>> {
	let table = String::from("firms");

	let firms_count = Count::count(&data.db, table).await.unwrap_or(0);

	for j in 0..=firms_count {
		println!("№ {}", &j);
		let firm = Firm::get_firm(&data.db, j).await.unwrap();

		let translit_name = Translit::convert(firm.name.clone());
		let firm_address = firm.address.clone().unwrap_or("".to_string());
		let firm_street = firm_address.split(",").collect::<Vec<&str>>()[0].to_string();
		let translit_address = if firm_address != "" {
			Translit::convert(Some(firm_street))
		} else {
			firm.firm_id.clone().to_string()
		};

		let mut firm_url = String::new();

		let firms_double_urls =
			sqlx::query_as!(Firm, r#"SELECT * FROM firms WHERE url = $1"#, &firm.url.clone().unwrap())
				.fetch_all(&data.db)
				.await?;

		if firms_double_urls.len() > 1 {
			firm_url = format!("{}-{}-{}", &translit_name, &translit_address, &firm.firm_id.clone());
		} else {
			firm_url = format!("{}-{}", &translit_name, &translit_address);
		}

		let _ = sqlx::query_as!(
			Firm,
			r#"UPDATE firms SET url = $1 WHERE firm_id = $2 RETURNING *"#,
			firm_url.replace(" ", "-")
			.replace(",", "-")
			.replace(".", "-")
			.replace("`", "")
			.replace("&amp;", "&"),
			firm.firm_id,
		)
		.fetch_one(&data.db)
		.await;

		dbg!(&firm_url);
	}

	Ok(())
}
