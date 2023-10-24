use crate::{jwt_auth, model::Quote, AppState};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::Duration;

#[get("/parser/quotes")]
async fn parser_handler(
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware
) -> impl Responder {
	let res = parser(data).await.unwrap();

	let json_response = serde_json::json!({
		"status":  "success",
		// "data": serde_json::json!({
		// 	"user": serde_json::to_string(&res)
		// })
	});

	HttpResponse::Ok().json(json_response)
}

async fn parser(data: web::Data<AppState>) -> WebDriverResult<Vec<(String, String)>> {
	let caps = DesiredCapabilities::chrome();
	let driver = WebDriver::new("http://localhost:9515", caps).await?;

	driver.goto("http://quotes.toscrape.com/scroll").await?;

	tokio::time::sleep(Duration::from_secs(2)).await;

	let mut quote_elems: Vec<WebElement> = Vec::new();

	for _n in 1..10 {
		quote_elems = driver.find_all(By::Css(".quote")).await?;
		let last = quote_elems.last().unwrap();
		last.scroll_into_view().await?;
		tokio::time::sleep(Duration::from_secs(1)).await;
	}

	let mut quotes = Vec::new();

	for quote_elem in quote_elems {
		let quote_text = quote_elem.find(By::Css(".text")).await?.text().await?;
		let author = quote_elem.find(By::Css(".author")).await?.text().await?;
		let quote = (quote_text, author);
		quotes.push(quote);
	}

	for quote in &quotes {
		let _ = sqlx::query_as!(
			Quote,
			"INSERT INTO quotes (text, author) VALUES ($1, $2) RETURNING *",
			quote.0.to_string(),
			quote.1.to_string(),
		)
		.fetch_one(&data.db)
		.await;

		println!("{} -- {}", quote.0, quote.1)
	}

	driver.quit().await?;

	Ok(quotes.clone())
}