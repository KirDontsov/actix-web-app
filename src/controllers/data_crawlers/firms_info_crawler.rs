use crate::{
	api::Driver,
	models::{Category, Firm, FirmsCount, SaveFirm, Type},
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
	let driver = <dyn Driver>::get_driver().await?;

	let firms_count = FirmsCount::count_firm(&data.db).await.unwrap_or(0);

	let category = sqlx::query_as!(
		Category,
		"SELECT * FROM categories WHERE abbreviation = 'car_service';",
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	let type_item = sqlx::query_as!(
		Type,
		"SELECT * FROM types WHERE abbreviation = 'kuzovnoj_remont';",
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;
	dbg!(&start);

	for j in start.clone()..=firms_count {
		let firm = Firm::get_firm(&data.db, j).await.unwrap();
		let mut firms: Vec<SaveFirm> = Vec::new();

		driver.goto(format!("https://2gis.ru/spb/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/firm/{}", &firm.two_gis_firm_id.clone().unwrap())).await?;
		sleep(Duration::from_secs(5)).await;

		// не запрашиваем информацию о закрытом
		let err_block = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div/div"))
			.first()
			.await?
			.inner_html()
			.await?;

		if err_block.contains("Филиал удалён из справочника")
			|| err_block.contains("Филиал временно не работает")
		{
			continue;
		}

		let info_blocks_xpath;
		let mut address_xpath;
		let mut phone_xpath;
		let mut site_xpath;
		// let mut email_xpath;

		// находим блоки среди которых есть блок с блоками с инфой
		let blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div")).all().await?;
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
		let info_blocks = driver.query(By::XPath(&info_blocks_xpath)).all().await?;
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
		dbg!(extra_block.contains("Показать телефон") || extra_block.contains("Показать телефоны"));
		// если нет телефона и сайта
		if !(extra_block.contains("Показать телефон") || extra_block.contains("Показать телефоны"))
		{
			phone_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[last()]", info_block_number);
		}
		// если нет сайта
		if info_blocks.len() <= 3 {
			site_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div[{}]/div/div[last()]", info_block_number);
		}

		let firm_address = info_blocks[0]
			.query(By::XPath(&address_xpath))
			.first()
			.await?
			.text()
			.await?;

		let firm_phone = match info_blocks[0]
			.query(By::XPath(&phone_xpath))
			.first()
			.await?
			.attr("href")
			.await?
		{
			Some(tel) => tel.replace("tel:", ""),
			_ => "-".to_string(),
		};

		let firm_site = info_blocks[0]
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
			type_id: type_item.type_id.clone(),
			// default_email: firm_email.clone(),
		});

		// запись в бд
		for firm in firms {
			let _ = sqlx::query_as!(
				Firm,
				"INSERT INTO firms (two_gis_firm_id, category_id, type_id, name, address, default_phone, site) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
				firm.two_gis_firm_id,
				firm.category_id,
				firm.type_id,
				firm.name,
				firm.address,
				firm.default_phone,
				firm.site,
			)
			.fetch_one(&data.db)
			.await;

			dbg!(&firm);
		}
		// обновляем в базе счетчик
		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;

		println!("№ {}", &j + 1);
	}

	driver.quit().await?;

	Ok(())
}
