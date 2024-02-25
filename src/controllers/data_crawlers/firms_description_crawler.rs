use crate::{
	api::Driver,
	jwt_auth,
	models::{Firm, FirmsCount, UpdateFirmDesc},
	utils::{get_counter, update_counter},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

#[allow(unreachable_code)]
#[get("/crawler/descriptions")]
async fn firms_description_crawler_handler(
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
	let counter_id: String = String::from("7711da84-7d98-4072-aa35-b642c7ac0762");
	let driver = <dyn Driver>::get_driver().await?;

	let firms_count = FirmsCount::count_firm(&data.db).await.unwrap_or(0);

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;

	for j in start.clone()..=firms_count {
		let firm = Firm::get_firm(&data.db, j).await.unwrap();
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

		if err_block.contains("Филиал удалён из справочника")
			|| err_block.contains("Филиал временно не работает")
		{
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
		// обновляем в базе счетчик
		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;

		println!("№ {}", &j + 1);
	}

	driver.clone().quit().await?;

	Ok(())
}
