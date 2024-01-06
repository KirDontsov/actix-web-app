use crate::{
	jwt_auth,
	models::{Category, Firm, FirmsCount, Review, SaveReview, TwoGisFirm},
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

		let mut reviews: Vec<SaveReview> = Vec::new();

		driver.goto(format!("https://2gis.ru/spb/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/firm/{}/tab/reviews", &firm.two_gis_firm_id.clone().unwrap())).await?;

		tokio::time::sleep(Duration::from_secs(5)).await;

		// не запрашиваем информацию если нет отзывов
		let err_block = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div"))
			.with_text("Нет отзывов").exists()
			.await?;

		if err_block {
			continue;
		}

		let mut block_number = 3;
		let mut not_confirmed;
		// let mut not_confirmed_xpath;
		let mut author_xpath;
		let mut date_xpath;
		let mut text_xpath;

		let initial_blocks = driver.find_all(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div")).await?;

		// проверяем дополнительный блок в начале перед проходм, чтобы не проверять на каждом шаге
		let extra_block = initial_blocks[3]
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]"))
			.first()
			.await?
			.inner_html()
			.await?;

		if extra_block.contains("Люди говорят") {
			block_number = 4;
		}

		not_confirmed = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]"))
			.first()
			.await?
			.inner_html()
			.await?;

		dbg!(&not_confirmed);

		// TODO: нужно считать кол-во отзывов / на 13 и по этому кол-ву скролить в цикле
		let last = initial_blocks.last().unwrap();
		last.scroll_into_view().await?;

		tokio::time::sleep(Duration::from_secs(1)).await;

		let blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div")).all().await?;
		dbg!(&blocks.len());

		for (i, block) in blocks.clone().into_iter().enumerate() {
			if i >= block_number {
				let count = i + 1;

				dbg!(&count);
				if not_confirmed.contains("Неподтвержденные отзывы") && count == 10
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

			dbg!(&review);
		}
	}

	driver.quit().await?;

	Ok(())
}
