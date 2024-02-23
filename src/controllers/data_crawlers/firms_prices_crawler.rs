use crate::{
	api::Driver,
	models::{Firm, FirmsCount, PriceCategory, PriceItem},
	utils::{get_counter, update_counter},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[allow(unreachable_code)]
#[get("/crawler/prices")]
async fn firms_prices_crawler_handler(
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
	let counter_id: String = String::from("5116c826-87a8-4881-ba9c-19c0068b3c62");

	let firms_count = FirmsCount::count_firm(&data.db).await.unwrap_or(0);

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;

	for j in start.clone()..=firms_count {
		let driver = <dyn Driver>::get_driver().await?;
		let firm = Firm::get_firm(&data.db, j).await.unwrap();

		println!("№ {}", &j);

		driver
			.goto(format!(
				"https://2gis.ru/spb/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/firm/{}/tab/prices",
				&firm.two_gis_firm_id.clone().unwrap()
			))
			.await?;
		sleep(Duration::from_secs(5)).await;

		let main_block = match find_main_block(driver.clone()).await {
			Ok(img_elem) => img_elem,
			Err(e) => {
				let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
				dbg!(&counter);
				println!("error while searching main block: {}", e);
				"".to_string()
			}
		};

		if main_block.contains("Филиал удалён из справочника")
			|| main_block.contains("Филиал временно не работает")
			|| main_block.contains("Добавьте сюда фотографий!")
		{
			continue;
		}

		let mut categories_blocks: Vec<WebElement> = Vec::new();
		let mut items_blocks: Vec<WebElement> = Vec::new();

		// кол-во цен
		let prices_count = match find_count_block(driver.clone()).await {
			Ok(img_elem) => img_elem,
			Err(e) => {
				let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
				dbg!(&counter);
				println!("error while searching count block: {}", e);
				0.0
			}
		};

		if prices_count == 0.0 {
			continue;
		}

		let edge: i32 = ((if prices_count > 500.0 {
			500.0
		} else {
			prices_count
		}) / 12.0)
			.ceil() as i32;

		// скролим в цикле
		for _ in 0..edge {
			let blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div[2]/div")).all().await?;
			let last = blocks.last().unwrap();
			last.scroll_into_view().await?;
			sleep(Duration::from_secs(2)).await;
		}

		categories_blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div[2]/div[contains(@class, \"_19i46pu\")]")).all().await?;
		items_blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div[2]/div[contains(@class, \"_8mqv20\")]")).all().await?;

		let mut total_count = 1;
		let mut items_by_category = 0;

		for i in 0..categories_blocks.len() {
			let category_count = i + 1;
			let category_id = Uuid::new_v4();
			let category_name = match find_element_by_xpath(driver.clone(), &format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div[2]/div[contains(@class, \"_19i46pu\")][{}]/div[1]", &category_count)).await {
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching image: {}", e);
					"".to_string()
				}
			};

			let category_value = match find_element_by_xpath(driver.clone(), &format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div[2]/div[contains(@class, \"_19i46pu\")][{}]/div[2]", &category_count)).await {
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching image: {}", e);
					"".to_string()
				}
			};

			if &category_name == "" || &category_value == "" {
				continue;
			}
			println!("Category №{}", &i);
			println!("Firm id: {}", firm.firm_id.clone());
			dbg!(&category_name);
			dbg!(&category_value);
			println!("{}", "======");

			// запись в бд
			let _ = sqlx::query_as!(
				PriceCategory,
				"INSERT INTO prices_categories (price_category_id, firm_id, name, value) VALUES ($1, $2, $3, $4) RETURNING *",
				category_id.clone(),
				firm.firm_id.clone(),
				category_name,
				category_value,
			)
			.fetch_one(&data.db)
			.await;

			if i == 0 {
				items_by_category = category_value.clone().parse::<i32>().unwrap();
			}

			println!("{}..{}", &total_count, &items_by_category);

			for i in total_count..=items_by_category.clone() {
				// TODO: опираясь на число айтемов в категории отсчитать нужное кол-во
				// и сохранить в их в базу со связью по category_id
				// потом прервать внутренний цикл
				// перейдя к следующей категории вычесть число записанных в предыдущей категории, и продолжить запись со следующего айтема

				let item_name = match find_element_by_xpath(driver.clone(), &format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div[2]/div[contains(@class, \"_8mqv20\")][{}]/div[1]", i)).await {
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching image: {}", e);
					"".to_string()
				}
			};

				let item_value = match find_element_by_xpath(driver.clone(), &format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div[2]/div[contains(@class, \"_8mqv20\")][{}]/div[2]", i)).await {
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching image: {}", e);
					"".to_string()
				}
			};

				if &item_name == "" || &item_value == "" {
					continue;
				}

				dbg!(&item_name);
				dbg!(&item_value);
				println!("{}", "======");

				// запись в бд
				let _ = sqlx::query_as!(
					PriceItem,
					"INSERT INTO prices_items (price_category_id, firm_id, name, value) VALUES ($1, $2, $3, $4) RETURNING *",
					category_id.clone(),
					firm.firm_id.clone(),
					item_name,
				  item_value,
				)
				.fetch_one(&data.db)
				.await;
			}

			total_count += category_value.clone().parse::<i32>().unwrap();
			if i == 0 {
				items_by_category += category_value.clone().parse::<i32>().unwrap() - 1;
			} else {
				items_by_category += category_value.clone().parse::<i32>().unwrap();
			}
		}

		// обновляем в базе счетчик
		// let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;

		println!("id: {}", &firm.two_gis_firm_id.clone().unwrap());
		println!("{}", "======");
		driver.clone().quit().await?;
	}

	Ok(())
}

pub async fn find_main_block(driver: WebDriver) -> Result<String, WebDriverError> {
	let err_block = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div"))
			.first()
			.await?
			.inner_html()
			.await?;

	Ok(err_block)
}

pub async fn find_count_block(driver: WebDriver) -> Result<f32, WebDriverError> {
	let prices_count = driver
			.query(By::XPath("//*[contains(text(),'Цены')]/span"))
			.or(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]"))
			.first()
			.await?
			.inner_html()
			.await?
			.parse::<f32>()
			.unwrap_or(0.0);

	Ok(prices_count)
}

pub async fn find_element_by_xpath(
	driver: WebDriver,
	xpath: &str,
) -> Result<String, WebDriverError> {
	let elem = driver
		.query(By::XPath(xpath))
		.first()
		.await?
		.inner_html()
		.await?;

	Ok(elem)
}
