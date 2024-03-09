use crate::{
	api::Driver,
	jwt_auth,
	models::{Count, Firm, Image},
	utils::{get_counter, update_counter},
	AppState,
};
use actix_web::{get, web, HttpResponse, Responder};
use std::{fs::create_dir, path::Path};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[allow(unreachable_code)]
#[get("/crawler/images")]
async fn firms_images_crawler_handler(
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
	let counter_id: String = String::from("2a94ecc5-fb8d-4b4d-bb03-e3ee2eb708da");
	let table = String::from("firms");
	let category_id = uuid::Uuid::parse_str("3ebc7206-6fed-4ea7-a000-27a74e867c9a").unwrap();

	let firms_count = Count::count_firms_by_category(&data.db, table, category_id)
		.await
		.unwrap_or(0);

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;

	for j in start.clone()..=firms_count {
		let driver = <dyn Driver>::get_driver().await?;
		let firm = Firm::get_firm(&data.db, j).await.unwrap();

		println!("№ {}", &j);

		driver
			.goto(format!(
				"https://2gis.ru/spb/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/firm/{}/tab/photos",
				&firm.two_gis_firm_id.clone().unwrap()
			))
			.await?;
		sleep(Duration::from_secs(5)).await;

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
			|| main_block.contains("Добавьте сюда фотографий!")
			|| main_block.contains("")
		{
			continue;
		}

		let mut blocks: Vec<WebElement> = Vec::new();

		// кол-во фото
		let img_count = match find_count_block(driver.clone()).await {
			Ok(img_elem) => img_elem,
			Err(e) => {
				let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
				dbg!(&counter);
				println!("error while searching count block: {}", e);
				0.0
			}
		};

		if img_count == 0.0 {
			continue;
		}

		let edge: i32 = ((if img_count > 100.0 { 50.0 } else { img_count }) / 12.0).ceil() as i32;

		// скролим в цикле
		for _ in 0..edge {
			blocks = driver.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]/div[2]/div[2]/div")).all().await?;
			let last = blocks.last().unwrap();
			last.scroll_into_view().await?;
			sleep(Duration::from_secs(2)).await;
		}

		let dir_name = format!("{}", &firm.firm_id.clone());
		let _ = create_dir(Path::new(format!("output/{}", &dir_name).as_str()))?;

		for (i, block) in blocks.clone().into_iter().enumerate() {
			let block_content = block.inner_html().await?;

			if block_content.contains("Добавить фото") {
				continue;
			}
			// Записываем в бд этот img_id, firm_id и можно сгенерить Alt для него
			let img_id = Uuid::new_v4();
			let file_name = format!("{}.jpg", &img_id);

			let img = match find_img(block).await {
				Ok(img_elem) => {
					println!("next");
					img_elem
				}
				Err(e) => {
					let counter = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
					dbg!(&counter);
					println!("error while searching image: {}", e);
					"".to_string()
				}
			};

			if &img == "" {
				break;
			}

			dbg!(&dir_name);
			dbg!(&img);

			// Get the current window handle.
			let handle = driver.window().await?;
			// Open a new tab.
			driver.new_tab().await?;

			// Get window handles and switch to the new tab.
			let handles = driver.windows().await?;
			driver.switch_to_window(handles[1].clone()).await?;

			// We are now controlling the new tab.
			driver.goto(&img.replace("_328x170", "")).await?;

			match driver
				.query(By::Tag("img"))
				.first()
				.await?
				.screenshot(Path::new(
					format!("{}/{}/{}", "output", dir_name, &file_name).as_str(),
				))
				.await
			{
				Ok(_) => println!("image saved successfully"),
				Err(e) => println!("error while downloading image: {}", e),
			};

			// Switch back to original tab.
			driver.switch_to_window(handle.clone()).await?;

			// запись в бд
			let _ = sqlx::query_as!(
				Image,
				"INSERT INTO images (img_id, firm_id, img_alt) VALUES ($1, $2, $3) RETURNING *",
				img_id.clone(),
				firm.firm_id.clone(),
				firm.name.clone().unwrap(),
			)
			.fetch_one(&data.db)
			.await;
		}

		// обновляем в базе счетчик
		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;

		println!("id: {}", &firm.two_gis_firm_id.clone().unwrap());
		println!("{}", "======");
		driver.clone().quit().await?;
	}

	Ok(())
}

pub async fn find_img(block: WebElement) -> Result<String, WebDriverError> {
	let img = block
		.query(By::Tag("img"))
		.first()
		.await?
		.attr("src")
		.await?
		.unwrap();
	Ok(img)
}

pub async fn find_main_block(driver: WebDriver) -> Result<String, WebDriverError> {
	let err_block = driver
			.query(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div"))
			.first()
			.await?
			.inner_html()
			.await?;

	Ok(err_block)
}
pub async fn find_count_block(driver: WebDriver) -> Result<f32, WebDriverError> {
	let img_count = driver
			.query(By::XPath("//*[contains(text(),'Фото')]/span"))
			.or(By::XPath("//body/div/div/div/div/div/div[2]/div[2]/div/div/div/div/div[2]/div[2]/div/div/div/div/div/div/div[2]"))
			.first()
			.await?
			.inner_html()
			.await?
			.parse::<f32>()
			.unwrap_or(0.0);

	Ok(img_count)
}
