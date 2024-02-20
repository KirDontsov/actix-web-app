use crate::{
	api::Driver,
	jwt_auth,
	models::{Counter, Firm, FirmsCount, Review, SaveReview},
	utils::{get_counter, update_counter},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use std::{
	fs::File,
	io::{copy, Cursor},
};
use thirtyfour::prelude::*;
use thiserror::Error;
use tokio::time::Duration;

#[get("/crawler/images_test")]
async fn firms_images_test_crawler_handler(
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
	let driver = <dyn Driver>::get_driver().await?;

	driver.goto("https://books.toscrape.com/").await?;

	tokio::time::sleep(Duration::from_secs(5)).await;

	let mut blocks: Vec<WebElement> = Vec::new();
	let mut img_xpath;
	// let mut reviews: Vec<SaveReview> = Vec::new();

	blocks = driver.query(By::XPath("//div/ol/li")).all().await?;

	for (i, block) in blocks.clone().into_iter().enumerate() {
		let count = i + 1;
		// let block_content = block.inner_html().await?;

		img_xpath = format!("//div/ol/li[{}]/article/div/a/img", count);

		let Some(img) = block
			.query(By::XPath(&img_xpath))
			.first()
			.await?
			.attr("src")
			.await?
		else {
			panic!("no src!");
		};

		dbg!(&img);

		let file_name = format!("output/rust-scrapper-{}.jpg", &i);
		let image_url = format!("https://books.toscrape.com/{}", &img);
		match download_image_to(&image_url, &file_name).await {
			Ok(_) => println!("image saved successfully"),
			Err(e) => println!("error while downloading image: {}", e),
		}
	}

	println!("{}", "======");

	driver.clone().quit().await?;

	Ok(())
}

async fn download_image_to(url: &str, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
	// Send an HTTP GET request to the URL
	let response = reqwest::get(url).await?;
	dbg!(&response);
	dbg!(&response.bytes().await?);
	// Create a new file to write the downloaded image to
	let mut file = File::create(file_name)?;

	Ok(())
}
