use crate::{
	api::Driver,
	models::{Count, Firm, Review, SaveReview},
	utils::{get_counter, update_counter},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

#[allow(unreachable_code)]
#[get("/crawler/reviews")]
async fn firms_reviews_crawler_handler(
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
	let counter_id: String = String::from("4bb99137-6c90-42e6-8385-83c522cde804");
	let table = String::from("firms");
	let city_id = uuid::Uuid::parse_str("566e11b5-79f5-4606-8c18-054778f3daf6").unwrap();
	let category_id = uuid::Uuid::parse_str("565ad1cb-b891-4185-ac75-24ab3898cf22").unwrap();
	let city = "moscow";

	let firms_count =
		Count::count_firms_by_city_category(&data.db, table.clone(), city_id, category_id)
			.await
			.unwrap_or(0);

	// получаем из базы начало счетчика
	let start: i64 = get_counter(&data.db, &counter_id).await;
	dbg!(&start);

	let driver = <dyn Driver>::get_driver().await?;

	for j in start.clone()..=firms_count {
		println!("№: {}", &j + 1);
		let firm =
			Firm::get_firm_by_city_category(&data.db, table.clone(), city_id, category_id, j)
				.await
				.unwrap();
		let mut reviews: Vec<SaveReview> = Vec::new();

		// проверка на дубликат
		let existed_reviews = sqlx::query_as!(
			Review,
			r#"SELECT * FROM reviews WHERE firm_id = $1;"#,
			&firm.firm_id
		)
		.fetch_one(&data.db)
		.await;

		dbg!(&existed_reviews);

		if existed_reviews.is_ok() {
			println!("{}", &firm.firm_id);
			println!("Already exists");
			continue;
		}

		driver.goto(format!("https://2gis.ru/{}/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/firm/{}/tab/reviews", &city, &firm.two_gis_firm_id.clone().unwrap())).await?;
		sleep(Duration::from_secs(5)).await;

		let no_reviews = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div"))
			.first()
			.await?
			.inner_html()
			.await?;

		if no_reviews.contains("Нет отзывов")
			|| no_reviews.contains("Филиал удалён из справочника")
			|| no_reviews.contains("Филиал временно не работает")
			|| no_reviews.contains("Скоро открытие")
		{
			continue;
		}

		// let mut not_confirmed_xpath;
		let mut author_xpath;
		let mut date_xpath;
		let mut text_xpath;
		let mut rating_xpath;

		let mut blocks: Vec<WebElement> = Vec::new();

		// кол-во отзывов
		let reviews_count = driver
			.query(By::XPath("//*[contains(text(),'Отзывы')]/span"))
			.or(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div"))
			.first()
			.await?
			.inner_html()
			.await?
			.parse::<f32>()
			.unwrap_or(0.0);

		if reviews_count == 0.0 {
			continue;
		}

		let edge: i32 = ((if reviews_count > 500.0 {
			100.0
		} else {
			reviews_count
		}) / 12.0)
			.ceil() as i32;

		// скролим в цикле
		for _ in 0..edge {
			blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div")).all().await?;
			let last = blocks.last().unwrap();
			last.scroll_into_view().await?;
			tokio::time::sleep(Duration::from_secs(1)).await;
		}

		for (i, block) in blocks.clone().into_iter().enumerate() {
			let count = i + 1;
			let block_content = block.inner_html().await?;

			if block_content.contains("Неподтвержденные отзывы")
				|| block_content.contains("Все отзывы")
				|| block_content.contains("Загрузить еще")
				|| block_content.contains("С ответами")
				|| block_content.contains("Люди говорят")
				|| block_content.contains("Оцените и оставьте отзыв")
				|| block_content.contains("оценки")
				|| block_content.contains("оценок")
				|| block_content.contains("оценка")
				|| block_content.contains("ответ")
				|| block_content.contains("/5")
			{
				continue;
			}

			author_xpath = format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[{}]/div[1]/div/div[1]/div[2]/span/span[1]/span", count );
			date_xpath = format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[{}]/div[1]/div/div[1]/div[2]/div", count );
			text_xpath = format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[{}]/div[3]/div/a", count );
			rating_xpath = format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[{}]/div/div/div[2]/div/div[1]/span", count );

			let author = match find_block(driver.clone(), author_xpath).await {
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching author block: {}", e);
					"".to_string()
				}
			};

			let date = match find_block(driver.clone(), date_xpath).await {
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching date block: {}", e);
					"".to_string()
				}
			};

			let text = match find_block(driver.clone(), text_xpath).await {
				Ok(elem) => elem,
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching text block: {}", e);
					"".to_string()
				}
			};

			let rating = match find_blocks(driver.clone(), rating_xpath).await {
				Ok(elem) => elem.to_string(),
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching text block: {}", e);
					"".to_string()
				}
			};

			reviews.push(SaveReview {
				firm_id: firm.firm_id.clone(),
				two_gis_firm_id: firm.two_gis_firm_id.clone().unwrap(),
				author: author.clone(),
				date: date.clone(),
				text: text.replace("\n", " "),
				rating,
			});
		}

		// запись в бд
		for review in reviews {
			let _ = sqlx::query_as!(
				Review,
				"INSERT INTO reviews (firm_id, two_gis_firm_id, author, date, text, rating, parsed) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
				review.firm_id,
				review.two_gis_firm_id,
				review.author,
				review.date,
				review.text,
				review.rating,
				true
			)
			.fetch_one(&data.db)
			.await;

			dbg!(&review);
		}
		// обновляем в базе счетчик
		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;

		println!("id: {}", &firm.two_gis_firm_id.clone().unwrap());
		println!("{}", "======");
	}
	driver.clone().quit().await?;

	Ok(())
}

pub async fn find_block(driver: WebDriver, xpath: String) -> Result<String, WebDriverError> {
	let block = driver
		.query(By::XPath(&xpath))
		.first()
		.await?
		.inner_html()
		.await?;

	Ok(block)
}

pub async fn find_blocks(driver: WebDriver, xpath: String) -> Result<usize, WebDriverError> {
	let length = driver.query(By::XPath(&xpath)).all().await?.len();
	Ok(length)
}
