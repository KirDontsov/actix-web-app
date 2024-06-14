use crate::{
	api::Driver,
	models::{Count, Firm, PriceCategory, PriceItem},
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
	let table = String::from("firms");
	let city_id = uuid::Uuid::parse_str("eb8a1f13-6915-4ac9-b7d5-54096a315d08").unwrap();
	let category_id = uuid::Uuid::parse_str("3ebc7206-6fed-4ea7-a000-27a74e867c9a").unwrap();
	let city = "spb";
	let category = "рестораны";

	let firms_count =
		Count::count_firms_by_city_category(&data.db, table.clone(), city_id, category_id)
			.await
			.unwrap_or(0);

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;
	let driver = <dyn Driver>::get_driver().await?;

	for j in start.clone()..=firms_count {
		let firm = Firm::get_firm_by_city_category(
			&data.db,
			table.clone(),
			city_id.clone(),
			category_id.clone(),
			j,
		)
		.await
		.unwrap();

		println!("№ {}", &j);

		// проверка на дубликат
		let existed_prices = sqlx::query_as!(
			PriceItem,
			r#"SELECT * FROM prices_items WHERE firm_id = $1;"#,
			&firm.firm_id
		)
		.fetch_one(&data.db)
		.await;

		if existed_prices.is_ok() {
			println!("{}", &firm.firm_id);
			println!("Already exists");
			continue;
		}

		driver
			.goto(format!(
				"https://2gis.ru/{}/search/{}/firm/{}/tab/prices",
				&city,
				&category,
				&firm.two_gis_firm_id.clone().unwrap()
			))
			.await?;
		sleep(Duration::from_secs(5)).await;

		let error_block = match find_error_block(driver.clone()).await {
			Ok(img_elem) => img_elem,
			Err(e) => {
				println!("error while searching error block: {}", e);
				"".to_string()
			}
		};

		if error_block.contains("Что-то пошло не так") {
			driver.refresh().await?;
		}

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
			|| main_block.contains("Посмотреть на сайте")
			|| main_block.contains("Добавьте сюда фотографий!")
		{
			continue;
		}

		let different_catalogue = match check_the_element(driver.clone(), "_183lbryc").await {
			Ok(elem) => elem,
			Err(e) => {
				let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
				dbg!(&counter);
				println!("error while searching _183lbryc: {}", e);
				driver.clone().quit().await?;
				vec![]
			}
		};

		let ads_catalogue = match check_the_element(driver.clone(), "_rixun1").await {
			Ok(elem) => elem,
			Err(e) => {
				let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
				dbg!(&counter);
				println!("error while searching _rixun1: {}", e);
				driver.clone().quit().await?;
				vec![]
			}
		};

		if different_catalogue.len() > 0 || ads_catalogue.len() > 0 {
			continue;
		}

		let mut categories_blocks: Vec<WebElement> = Vec::new();

		// кол-во цен
		let prices_count = match find_count_block(driver.clone()).await {
			Ok(img_elem) => img_elem,
			Err(e) => {
				let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
				dbg!(&counter);
				println!("error while searching count block: {}", e);
				driver.clone().quit().await?;
				0.0
			}
		};

		if prices_count == 0.0 {
			continue;
		}

		let edge = (prices_count / 20.0).ceil() as i32;

		dbg!(&prices_count);
		dbg!(&edge);

		// скролим в цикле
		for _ in 0..edge {
			let blocks = match find_elements_by_class(driver.clone(), "_8mqv20").await {
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching category: {}", e);
					driver.clone().quit().await?;
					vec![]
				}
			};

			let first = blocks.first().unwrap();
			let last = blocks.last().unwrap();

			last.scroll_into_view().await?;
			sleep(Duration::from_secs(2)).await;

			first.scroll_into_view().await?;
			sleep(Duration::from_secs(1)).await;
		}

		categories_blocks = match find_elements_by_class(driver.clone(), "_19i46pu").await {
			Ok(elem) => elem,
			Err(e) => {
				let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
				dbg!(&counter);
				println!("error while searching category: {}", e);
				driver.clone().quit().await?;
				vec![]
			}
		};

		let mut total_count = 1;
		let mut items_by_category = 0;

		for i in 0..categories_blocks.len() {
			let category_count = i + 1;
			let category_id = Uuid::new_v4();
			let category_name = match find_element_by_xpath(
				driver.clone(),
				&format!(
					"//div[contains(@class, \"_19i46pu\")][{}]/div[1]",
					&category_count
				),
				&format!(
					"//div[contains(@class, \"_19i46pu\")][{}]/div",
					&category_count
				),
			)
			.await
			{
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching category: {}", e);
					driver.clone().quit().await?;
					"".to_string()
				}
			};

			if &category_name == "" {
				continue;
			}

			let category_value = match find_element_by_xpath(
				driver.clone(),
				&format!(
					"//div[contains(@class, \"_19i46pu\")][{}]/div[2]",
					&category_count
				),
				&format!(
					"//div[contains(@class, \"_19i46pu\")][{}]/div[last()]",
					&category_count
				),
			)
			.await
			{
				Ok(elem) => elem,
				Err(e) => {
					println!("error while searching category: {}", e);
					"".to_string()
				}
			};

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
				items_by_category = category_value.clone().parse::<i32>().unwrap_or(0);
			}

			println!("{}..{}", &total_count, &items_by_category);

			for i in total_count..=items_by_category.clone() {
				// TODO: опираясь на число айтемов в категории отсчитать нужное кол-во
				// и сохранить в их в базу со связью по category_id
				// потом прервать внутренний цикл
				// перейдя к следующей категории вычесть число записанных в предыдущей категории, и продолжить запись со следующего айтема

				if i > prices_count.ceil() as i32 {
					continue;
				}

				let item_name = match find_element_by_xpath(
					driver.clone(),
					&format!("//div[contains(@class, \"_8mqv20\")][{}]/div[1]", i),
					&format!("//div[contains(@class, \"_8mqv20\")][{}]/div", i),
				)
				.await
				{
					Ok(elem) => elem,
					Err(e) => {
						let counter =
							update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
						dbg!(&counter);
						println!("error while searching prices: {}", e);
						driver.clone().quit().await?;
						"".to_string()
					}
				};

				if &item_name == "" {
					continue;
				}

				let item_value = match find_element_by_xpath(
					driver.clone(),
					&format!("//div[contains(@class, \"_8mqv20\")][{}]/div[2]", i),
					&format!("//div[contains(@class, \"_8mqv20\")][{}]/div[last()]", i),
				)
				.await
				{
					Ok(elem) => elem,
					Err(e) => {
						println!("error while searching prices: {}", e);
						"".to_string()
					}
				};

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

			total_count += category_value.clone().parse::<i32>().unwrap_or(0);
			if i < categories_blocks.len() {
				items_by_category += category_value.clone().parse::<i32>().unwrap_or(0) - 1;
			}
		}

		// обновляем в базе счетчик
		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;

		println!("id: {}", &firm.two_gis_firm_id.clone().unwrap());
		println!("{}", "======");
	}

	driver.clone().quit().await?;

	Ok(())
}

pub async fn find_main_block(driver: WebDriver) -> Result<String, WebDriverError> {
	let err_block = driver
		.query(By::ClassName("_18lzknl"))
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
	xpath2: &str,
) -> Result<String, WebDriverError> {
	let elem = driver
		.query(By::XPath(xpath))
		.or(By::XPath(xpath2))
		.first()
		.await?
		.inner_html()
		.await?;

	Ok(elem)
}

pub async fn find_elements_by_class(
	driver: WebDriver,
	classname: &str,
) -> Result<Vec<WebElement>, WebDriverError> {
	let elem = driver
		.query(By::ClassName(classname))
		.all_from_selector_required()
		.await?;

	Ok(elem)
}

pub async fn check_the_element(
	driver: WebDriver,
	classname: &str,
) -> Result<Vec<WebElement>, WebDriverError> {
	let elem = driver
		.query(By::ClassName(classname))
		.nowait()
		.all_from_selector_required()
		.await?;

	Ok(elem)
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
