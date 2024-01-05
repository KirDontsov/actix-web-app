use crate::{
	jwt_auth,
	models::{Category, Firm, FirmsCount, SaveFirm, TwoGisFirm},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::Duration;

#[get("/crawler/firms_info")]
async fn firms_info_crawler_handler(
	data: web::Data<AppState>,
	_: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let _ = crawler(data).await;

	let json_response = serde_json::json!({
		"status":  "success",
	});

	HttpResponse::Ok().json(json_response)
}

async fn crawler(data: web::Data<AppState>) -> WebDriverResult<()> {
	let caps = DesiredCapabilities::chrome();
	let driver = WebDriver::new("http://localhost:9515", caps).await?;

	let count_query_result =
		sqlx::query_as!(FirmsCount, "SELECT count(*) AS count FROM two_gis_firms")
			.fetch_one(&data.db)
			.await;

	if count_query_result.is_err() {
		println!("Что-то пошло не так во время подсчета фирм");
	}

	let firms_count = count_query_result.unwrap().count.unwrap();

	dbg!(&firms_count);

	let category = sqlx::query_as!(
		Category,
		"SELECT * FROM categories WHERE name = 'car_service';",
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	for j in 0..=firms_count {
		let firm = sqlx::query_as!(
			TwoGisFirm,
			"SELECT * FROM two_gis_firms ORDER BY two_gis_firm_id LIMIT 1 OFFSET $1;",
			j
		)
		.fetch_one(&data.db)
		.await
		.unwrap();

		let mut firms: Vec<SaveFirm> = Vec::new();

		driver.goto(format!("https://2gis.ru/spb/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/firm/{}/30.384797%2C59.980609", &firm.two_gis_firm_id.clone().unwrap())).await?;

		tokio::time::sleep(Duration::from_secs(5)).await;

		let address_xpath;
		let mut phone_xpath;
		let mut site_xpath;
		// let mut email_xpath;

		address_xpath = "//body/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div/div";
		// attribute
		phone_xpath = "//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[1]/div/div/div[3]/div[2]/div/a";
		site_xpath = "//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[1]/div/div/div[4]";
		// email_xpath = "//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div/div/div[6]/div[2]/div/a";

		// не запрашиваем информацию о закрытом
		let err_block = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div/div"))
			.with_text("Филиал временно не работает").exists()
			.await?;

		if err_block {
			continue;
		}

		let blocks = driver.find_all(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[1]/div")).await?;

		let mut target_block = blocks[0].clone();
		for block in blocks {
			if block.rect().await?.height >= target_block.rect().await?.height {
				target_block = block.clone();
			}
		}

		let info_blocks = driver.find_all(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[1]/div/div/div")).await?;
		let mut info_blocks_count = 0;

		for (i, _) in info_blocks.into_iter().enumerate() {
			info_blocks_count = i;
		}

		if info_blocks_count <= 3 {
			site_xpath = "//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[1]/div/div/div[last()]";
		}

		if info_blocks_count <= 3 {
			phone_xpath = "//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[1]/div/div/div[last()]/div[last()]";
		}

		let firm_address = target_block
			.query(By::XPath(&address_xpath))
			.first()
			.await?
			.text()
			.await?;

		let firm_phone = match target_block
			.query(By::XPath(&phone_xpath))
			.first()
			.await?
			.attr("href")
			.await?
		{
			Some(tel) => tel.replace("tel:", ""),
			_ => "-".to_string(),
		};

		let firm_site = target_block
			.query(By::XPath(&site_xpath))
			.first()
			.await?
			.text()
			.await?;

		// let firm_email = firm_elem
		// 	.find(By::XPath(&email_xpath))
		// 	.await?
		// 	.inner_html()
		// 	.await?;

		firms.push(SaveFirm {
			two_gis_firm_id: firm.two_gis_firm_id.clone().unwrap(),
			category_id: category.category_id.clone(),
			name: firm.name.clone().unwrap(),
			address: firm_address.replace("\n", ", "),
			default_phone: firm_phone.clone(),
			site: firm_site.clone(),
			// default_email: firm_email.clone(),
		});

		// запись в бд
		for firm in firms {
			let _ = sqlx::query_as!(
				Firm,
				"INSERT INTO firms (two_gis_firm_id, category_id, name, address, default_phone, site) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
				firm.two_gis_firm_id,
				firm.category_id,
				firm.name,
				firm.address,
				firm.default_phone,
				firm.site,
			)
			.fetch_one(&data.db)
			.await;

			dbg!(&firm);
		}
	}

	driver.quit().await?;

	Ok(())
}
