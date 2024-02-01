use crate::{
	jwt_auth,
	models::{Firm, FirmsCount, UpdateFirmDesc},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

#[get("/crawler/firms_description")]
async fn firms_description_crawler_handler(
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let _ = crawler(data).await;

	let json_response = serde_json::json!({
		"status":"firms_description success",
	});

	HttpResponse::Ok().json(json_response)
}

async fn crawler(data: web::Data<AppState>) -> WebDriverResult<()> {
	let caps = DesiredCapabilities::chrome();
	let driver = WebDriver::new("http://localhost:9515", caps).await?;

	let count_query_result = sqlx::query_as!(FirmsCount, "SELECT count(*) AS count FROM firms")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		println!("Что-то пошло не так во время подсчета фирм");
	}

	let firms_count = count_query_result.unwrap().count.unwrap();

	dbg!(&firms_count);

	for j in 0..=firms_count {
		let firm = sqlx::query_as!(
			Firm,
			"SELECT * FROM firms ORDER BY two_gis_firm_id LIMIT 1 OFFSET $1;",
			j
		)
		.fetch_one(&data.db)
		.await
		.unwrap();

		let mut firms: Vec<UpdateFirmDesc> = Vec::new();

		driver.goto(format!("https://2gis.ru/spb/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/firm/{}/tab/info", &firm.two_gis_firm_id.clone().unwrap())).await?;

		sleep(Duration::from_secs(5)).await;

		// не запрашиваем информацию о закрытом
		let err_block = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div/div"))
			.first()
			.await?
			.inner_html()
			.await?;

		if err_block.contains("Филиал временно не работает") {
			continue;
		}

		let desc_block_xpath;

		// находим блоки среди которых есть блок с блоками с инфой
		let info_blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div/div/div")).all().await?;
		// находим номер блока с блоками с инфой
		let mut info_block_number = 1;
		for (i, block) in info_blocks.clone().into_iter().enumerate() {
			if block.rect().await?.height >= info_blocks[0].rect().await?.height
				&& !(block.inner_html().await?.contains("Авторемонт")
					|| block
						.inner_html()
						.await?
						.contains("Продажа легковых автомобилей")
					|| block.inner_html().await?.contains("Кузовной ремонт")
					|| block
						.inner_html()
						.await?
						.contains("Автозапчасти и аксессуары")
					|| block
						.inner_html()
						.await?
						.contains("Марки легковых запчастей")
					|| block
						.inner_html()
						.await?
						.contains("Ремонт ходовой части автомобиля")
					|| block.inner_html().await?.contains("Способы оплаты")
					|| block.inner_html().await?.contains("В справочнике")
					|| block.inner_html().await?.contains("Рядом")
					|| block.inner_html().await?.contains("Транспорт"))
			{
				info_block_number = i + 1;
			}
		}
		desc_block_xpath = format!("//body/div/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div[last()]/div[last()]/div/div/div/div/div/div/div[last()]/div[2]/div[1]/div/div/div[{}]", info_block_number);
		dbg!(&info_block_number);
		dbg!(&desc_block_xpath);

		dbg!(&info_blocks.len());

		let firm_desc = info_blocks[0]
			.query(By::XPath(&desc_block_xpath))
			.first()
			.await?
			.text()
			.await?;

		firms.push(UpdateFirmDesc {
			firm_id: firm.firm_id.clone(),
			description: firm_desc.clone().replace("\n", ", "),
		});

		// запись в бд
		for firm in firms {
			let _ = sqlx::query_as!(
				Firm,
				r#"UPDATE firms SET description = $1 WHERE firm_id = $2 RETURNING *"#,
				firm.description,
				firm.firm_id,
			)
			.fetch_one(&data.db)
			.await;

			dbg!(&firm);
		}
		println!("№ {}", &j + 1);
	}

	driver.clone().quit().await?;

	Ok(())
}
