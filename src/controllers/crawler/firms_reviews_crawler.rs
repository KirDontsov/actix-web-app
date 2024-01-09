use crate::{
	jwt_auth,
	models::{Firm, FirmsCount, Review, SaveReview},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::Duration;

#[get("/crawler/firms_reviews")]
async fn firms_reviews_crawler_handler(
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
		sqlx::query_as!(FirmsCount, "SELECT count(*) AS count FROM firms_copy")
			.fetch_one(&data.db)
			.await;

	if count_query_result.is_err() {
		println!("Что-то пошло не так во время подсчета фирм");
	}

	let firms_count = count_query_result.unwrap().count.unwrap();

	for j in 0..=firms_count {
		let firm = sqlx::query_as!(
			Firm,
			"SELECT * FROM firms_copy ORDER BY two_gis_firm_id LIMIT 1 OFFSET $1;",
			j
		)
		.fetch_one(&data.db)
		.await
		.unwrap();

		let mut reviews: Vec<SaveReview> = Vec::new();

		driver.goto(format!("https://2gis.ru/spb/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/firm/{}/tab/reviews", &firm.two_gis_firm_id.clone().unwrap())).await?;

		tokio::time::sleep(Duration::from_secs(5)).await;

		// let mut not_confirmed_xpath;
		let mut author_xpath;
		let mut date_xpath;
		let mut text_xpath;

		let mut blocks: Vec<WebElement> = Vec::new();

		let no_reviews = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]"))
			.first()
			.await?
			.inner_html()
			.await?;

		if no_reviews.contains("Нет отзывов") {
			continue;
		}

		// кол-во отзывов
		let reviews_count = driver
			.query(By::XPath("//*[contains(text(),'Отзывы')]/span"))
			.first()
			.await?
			.inner_html()
			.await?
			.parse::<f32>()
			.unwrap();

		let edge: i32 = ((if reviews_count > 500.0 {
			500.0
		} else {
			reviews_count
		}) / 12.0)
			.ceil() as i32;

		// скролим в цикле
		for _ in 0..=edge {
			blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div")).all().await?;
			let last = blocks.last().unwrap();
			last.scroll_into_view().await?;
			tokio::time::sleep(Duration::from_secs(1)).await;
		}

		for (i, block) in blocks.clone().into_iter().enumerate() {
			let count = i + 1;
			let block_content = block.inner_html().await?;

			if block_content.contains("Неподтвержденные отзывы")
				|| block_content.contains("Загрузить еще")
				|| block_content.contains("С ответами")
				|| block_content.contains("Люди говорят")
				|| block_content.contains("Оцените и оставьте отзыв")
				|| block_content.contains("/5")
			{
				continue;
			}

			author_xpath = format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[{}]/div[1]/div/div[1]/div[2]/span/span[1]/span", count );
			date_xpath = format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[{}]/div[1]/div/div[1]/div[2]/div", count );
			text_xpath = format!("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[{}]/div[3]/div/a", count );

			let author = block
				.query(By::XPath(&author_xpath))
				.first()
				.await?
				.inner_html()
				.await?;

			let date = block
				.query(By::XPath(&date_xpath))
				.first()
				.await?
				.inner_html()
				.await?;

			let text = block
				.query(By::XPath(&text_xpath))
				.first()
				.await?
				.inner_html()
				.await?;

			reviews.push(SaveReview {
				firm_id: firm.firm_id.clone(),
				two_gis_firm_id: firm.two_gis_firm_id.clone().unwrap(),
				author: author.clone(),
				date: date.clone(),
				text: text.replace("\n", " "),
			});
		}

		// запись в бд
		for review in reviews {
			let _ = sqlx::query_as!(
				Review,
				"INSERT INTO reviews (firm_id, two_gis_firm_id, author, date, text) VALUES ($1, $2, $3, $4, $5) RETURNING *",
				review.firm_id,
				review.two_gis_firm_id,
				review.author,
				review.date,
				review.text,
			)
			.fetch_one(&data.db)
			.await;
		}

		println!("№: {}", &j + 1);
		println!("id: {}", &firm.two_gis_firm_id.clone().unwrap());
		println!("{}", "======");
	}

	driver.quit().await?;

	Ok(())
}
