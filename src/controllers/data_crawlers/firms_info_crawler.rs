use crate::{
	api::Driver,
	models::{Category, Count, Firm, SaveFirm, TwoGisFirm, Type},
	utils::{get_counter, update_counter},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

#[allow(unreachable_code)]
#[get("/crawler/infos")]
async fn firms_info_crawler_handler(
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
	let counter_id: String = String::from("55d7ef92-45ca-40df-8e88-4e1a32076367");
	let table = String::from("two_gis_firms");
	let city_id = uuid::Uuid::parse_str("eb8a1f13-6915-4ac9-b7d5-54096a315d08").unwrap();
	let category_id = uuid::Uuid::parse_str("cc1492f6-a484-4c5f-b570-9bd3ec793613").unwrap();
	let city = "spb";
	let category_name = "клуб";
	let rubric_id = "173";

	let driver = <dyn Driver>::get_driver().await?;

	let firms_count = Count::count(&data.db, table).await.unwrap_or(0);

	let category = sqlx::query_as!(
		Category,
		"SELECT * FROM categories WHERE abbreviation = 'clubs';",
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	let type_item = sqlx::query_as!(
		Type,
		"SELECT * FROM types WHERE abbreviation = 'shinomontazh';",
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;
	dbg!(&start);

	for j in start.clone()..=firms_count {
		let firm = sqlx::query_as!(
			TwoGisFirm,
			"SELECT * FROM two_gis_firms ORDER BY two_gis_firm_id LIMIT 1 OFFSET $1;",
			j
		)
		.fetch_one(&data.db)
		.await
		.unwrap();

		driver
			.goto(format!(
				"https://2gis.ru/{}/search/{}/rubricId/{}/firm/{}",
				&city,
				&category_name,
				&rubric_id,
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

		// не запрашиваем информацию о закрытом
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
			|| main_block.contains("Скоро открытие")
		{
			continue;
		}

		let info_blocks_xpath;
		let mut address_xpath;
		let mut phone_xpath;
		let mut site_xpath;
		// let mut email_xpath;

		// находим блоки среди которых есть блок с блоками с инфой
		let blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div")).all_from_selector_required().await?;
		// находим номер блока с блоками с инфой
		let mut info_block_number = 1;
		for (i, block) in blocks.clone().into_iter().enumerate() {
			if block.rect().await?.height >= blocks[0].rect().await?.height
				&& (block.inner_html().await?.contains("Показать вход")
					|| block.inner_html().await?.contains("Показать на карте"))
			{
				info_block_number = i + 1;
			}
		}
		info_blocks_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div", info_block_number);

		// находим блоки с инфой
		let info_blocks = driver
			.query(By::XPath(&info_blocks_xpath))
			.all_from_selector_required()
			.await?;
		// есть ли доп блок "Уже воспользовались услугами?"
		let extra_block = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[1]"))
			.first()
			.await?
			.inner_html()
			.await?;

		// без доп блока
		address_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[{}]/div/div[1]", info_block_number, 1);
		phone_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[{}]/div[last()]/div/a", info_block_number, 3);
		site_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[{}]", info_block_number, 4);
		// email_xpath = "//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div/div/div/div[6]/div[2]/div/a";

		// с доп блоком
		if extra_block.contains("Уже воспользовались услугами?") {
			address_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[{}]/div/div[1]/div[2]/div[1]", info_block_number, 2);
			phone_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[{}]/div[last()]/div/a", info_block_number, 4);
			site_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[{}]", info_block_number, 5);
		}

		// если нет телефона и сайта
		if !(extra_block.contains("Показать телефон") || extra_block.contains("Показать телефоны"))
		{
			phone_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[last()]", info_block_number);
		}
		// если нет сайта
		if info_blocks.len() <= 3 {
			site_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[last()]", info_block_number);
		}

		let firm_address = match find_text_block(driver.clone(), address_xpath).await {
			Ok(elem) => elem,
			Err(e) => {
				println!("error while searching firm_address block: {}", e);
				"".to_string()
			}
		};

		let firm_phone = match find_tag_block(driver.clone(), phone_xpath).await {
			Ok(elem) => elem.replace("tel:", ""),
			Err(e) => {
				println!("error while searching firm_phone block: {}", e);
				"".to_string()
			}
		};

		let firm_site = match find_text_block(driver.clone(), site_xpath).await {
			Ok(elem) => elem,
			Err(e) => {
				println!("error while searching firm_site block: {}", e);
				"".to_string()
			}
		};

		let existed_firm =
			sqlx::query_as::<_, Firm>("SELECT * FROM firms WHERE two_gis_firm_id = $1")
				.bind(firm.two_gis_firm_id.clone().unwrap())
				.fetch_one(&data.db)
				.await;

		dbg!(&existed_firm);

		// запись в бд
		if existed_firm.is_ok() {
			println!("UPDATE {}", firm.two_gis_firm_id.clone().unwrap());

			let _ = sqlx::query_as::<_, Firm>(
				r#"UPDATE firms SET coords = $1 WHERE two_gis_firm_id = $2 RETURNING *"#,
			)
			.bind(firm.coords.clone().unwrap())
			.bind(firm.two_gis_firm_id.clone().unwrap())
			.fetch_one(&data.db)
			.await;
		} else {
			println!("INSERT {}", firm.two_gis_firm_id.clone().unwrap());

			let _ = sqlx::query_as::<_, Firm>(
				"INSERT INTO firms (two_gis_firm_id, city_id, category_id, type_id, name, address, default_phone, site, coords) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *"
			)
			.bind(firm.two_gis_firm_id.clone().unwrap())
			.bind(city_id.clone())
			.bind(category.category_id.clone())
			.bind(type_item.type_id.clone())
			.bind(firm.name.clone().unwrap())
			.bind(firm_address.replace("\n", ", "))
			.bind(firm_phone.clone())
			.bind(firm_site.clone())
			.bind(firm.coords.clone().unwrap())
			.fetch_one(&data.db)
			.await;
		}

		// обновляем в базе счетчик
		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;

		println!("№ {}", &j + 1);
	}

	driver.quit().await?;

	Ok(())
}

pub async fn find_main_block(driver: WebDriver) -> Result<String, WebDriverError> {
	let block = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div"))
			.first()
			.await?
			.inner_html()
			.await?;

	Ok(block)
}

pub async fn find_text_block(driver: WebDriver, xpath: String) -> Result<String, WebDriverError> {
	let block = driver
		.query(By::XPath(&xpath))
		.first()
		.await?
		.text()
		.await?;

	Ok(block)
}

pub async fn find_tag_block(driver: WebDriver, xpath: String) -> Result<String, WebDriverError> {
	let block = driver
		.query(By::XPath(&xpath))
		.first()
		.await?
		.attr("href")
		.await?
		.unwrap_or("".to_string());

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
