use crate::{
	api::Driver,
	jwt_auth,
	models::{Count, Firm, UpdateFirmAddress},
	utils::{get_counter, update_counter},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

#[allow(unreachable_code)]
#[get("/crawler/address")]
async fn firms_address_crawler_handler(
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
	let counter_id: String = String::from("1e69083b-ef25-43d6-8a08-8e1d2673826e");
	let table = String::from("firms");
	let city = "moscow";
	let category = "рестораны";
	let empty_field = "address".to_string();
	let driver = <dyn Driver>::get_driver().await?;

	let firms_count =
		Count::count_firms_with_empty_field(&data.db, table.clone(), empty_field.clone())
			.await
			.unwrap_or(0);

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;

	for j in start.clone()..=firms_count {
		let firm = Firm::get_firm_with_empty_field(&data.db, table.clone(), empty_field.clone(), j)
			.await
			.unwrap();
		let mut firms: Vec<UpdateFirmAddress> = Vec::new();

		let url = format!(
			"https://2gis.ru/{}/search/{}/firm/{}",
			&city,
			&category,
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

		let blocks = match find_address_blocks(
			driver.clone(),
			"//div[contains(@class, \"_49kxlr\")]".to_string(),
		)
		.await
		{
			Ok(elem) => elem,
			Err(e) => {
				println!("error while searching firm_site block: {}", e);
				driver.clone().quit().await?;
				[].to_vec()
			}
		};

		let mut address = "".to_string();

		for block in blocks {
			let block_content = block.inner_html().await?;
			if block_content.contains("Оформить") {
				continue;
			}

			// if block_content.contains("этаж")
			// 	|| block_content.contains("Москва")
			// 	|| block_content.contains("Санкт-Петербург")
			// {
			// 	address = block.text().await?;
			// 	break;
			// }
			address = block.text().await?;
			break;
		}

		firms.push(UpdateFirmAddress {
			firm_id: firm.firm_id.clone(),
			address: address.clone().replace("\n", ", "),
		});

		// запись в бд
		for firm in firms {
			let _ = sqlx::query_as::<_, Firm>(
				r#"UPDATE firms SET address = $1 WHERE firm_id = $2 RETURNING *"#,
			)
			.bind(&firm.address)
			.bind(&firm.firm_id)
			.fetch_one(&data.db)
			.await;

			dbg!(&firm);
		}
		// обновляем в базе счетчик
		// let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;

		println!("№ {}", &j + 1);
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

pub async fn find_address_blocks(
	driver: WebDriver,
	xpath: String,
) -> Result<Vec<WebElement>, WebDriverError> {
	let block = driver
		.query(By::XPath(&xpath))
		.all_from_selector_required()
		.await?;

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
