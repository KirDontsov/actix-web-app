use crate::{
	api::Driver,
	jwt_auth,
	models::{Count, Firm, UpdateFirmRating},
	utils::{get_counter, update_counter},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

#[allow(unreachable_code)]
#[get("/crawler/rating")]
async fn firms_rating_crawler_handler(
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	loop {
		let mut needs_to_restart = true;
		if needs_to_restart {
			let _: Result<(), Box<dyn std::error::Error>> = match crawler(data.clone()).await {
				Ok(x) => {
					needs_to_restart = false;
					Ok(x)
				}
				Err(e) => {
					println!("{:?}", e);
					needs_to_restart = true;
					Err(Box::new(e))
				}
			};
		}
	}
	let json_response = serde_json::json!({
		"status":  "success",
	});
	HttpResponse::Ok().json(json_response)
}

async fn crawler(data: web::Data<AppState>) -> WebDriverResult<()> {
	let counter_id: String = String::from("ff8641c7-8956-4d5d-bd45-4f90633415e6");
	let table = String::from("firms");
	let city_id = uuid::Uuid::parse_str("eb8a1f13-6915-4ac9-b7d5-54096a315d08").unwrap();
	let category_id = uuid::Uuid::parse_str("cc1492f6-a484-4c5f-b570-9bd3ec793613").unwrap();
	let city = "spb";
	let category_name = "клуб";
	let rubric_id = "173";

	let empty_field = "rating".to_string();

	let firms_count =
		Count::count_firms_with_empty_field(&data.db, table.clone(), empty_field.clone())
			.await
			.unwrap_or(0);

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;

	let driver = <dyn Driver>::get_driver().await?;

	for j in start.clone()..=firms_count {
		println!("№ {}", &j + 1);

		let firm = Firm::get_firm_with_empty_field(&data.db, table.clone(), empty_field.clone(), j)
			.await
			.unwrap();
		let mut firms: Vec<UpdateFirmRating> = Vec::new();

		let url = format!(
			"https://2gis.ru/{}/search/{}/firm/{}",
			&city,
			&category_name,
			&firm.two_gis_firm_id.clone().unwrap()
		);

		driver.goto(url).await?;
		sleep(Duration::from_secs(5)).await;

		let error_block = match find_error_block(driver.clone()).await {
			Ok(img_elem) => img_elem,
			Err(e) => {
				println!("error while searching error block: {}", e);
				driver.clone().quit().await?;
				"".to_string()
			}
		};

		if error_block.contains("Что-то пошло не так") {
			driver.refresh().await?;
		}

		// _y10azs

		let rating = match find_rating_blocks(driver.clone(), "//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[1]/div[4]/div/div[2]".to_string(), "//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[1]/div[4]/div/div[contains(@class, \"_y10azs\")]".to_string()).await {
			Ok(elem) => {
				elem.replace("Реклама", "").replace("Заказать доставку
Заказать онлайн", "").replace("Заказать доставку", "").replace("Заказать онлайн", "").replace("Забронировать онлайн", "").replace("Забронировать", "")
			},
			Err(e) => {
				let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
				dbg!(&counter);
				println!("error while searching text block: {}", e);
				driver.clone().quit().await?;
				"".to_string()
			}
		};

		println!("{}", rating.clone());

		// запись в бд
		let _ = sqlx::query_as::<_, Firm>(
			r#"UPDATE firms SET rating = $1 WHERE firm_id = $2 RETURNING *"#,
		)
		.bind(rating.clone())
		.bind(&firm.firm_id)
		.fetch_one(&data.db)
		.await;

		// обновляем в базе счетчик
		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
	}

	driver.clone().quit().await?;

	Ok(())
}

pub async fn find_block(driver: WebDriver, xpath: String) -> Result<String, WebDriverError> {
	let block = driver
		.query(By::XPath(&xpath))
		.first()
		.await?
		.text()
		.await?;

	Ok(block)
}

pub async fn find_rating_blocks(
	driver: WebDriver,
	xpath: String,
	second_xpath: String,
) -> Result<String, WebDriverError> {
	let exists = driver
		.query(By::XPath(&xpath))
		.or(By::XPath(&second_xpath))
		.exists()
		.await?;

	let mut block = String::new();

	if exists {
		block = driver
			.query(By::XPath(&xpath))
			.or(By::XPath(&second_xpath))
			.first()
			.await?
			.text()
			.await?;
	}

	Ok(block)
}

pub async fn find_error_block(driver: WebDriver) -> Result<String, WebDriverError> {
	let err_block = driver
		.query(By::Id("root"))
		.first()
		.await?
		.inner_html()
		.await?;

	Ok(err_block)
}
