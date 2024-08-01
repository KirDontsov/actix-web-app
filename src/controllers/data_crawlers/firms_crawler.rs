use crate::{api::Driver, models::TwoGisFirm, AppState};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

#[get("/crawler/firms")]
async fn firms_crawler_handler(
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let _ = crawler(data).await;

	let json_response = serde_json::json!({
		"status":  "success",
	});

	HttpResponse::Ok().json(json_response)
}

async fn crawler(data: web::Data<AppState>) -> WebDriverResult<()> {
	let driver = <dyn Driver>::get_driver().await?;
	let city = "moscow";
	let category_name = "школы";
	let rubric_id = "245";

	// автосервисы
	let url = format!(
		"https://2gis.ru/{}/search/{}/rubricId/{}",
		&city, &category_name, &rubric_id
	);
	// рестораны
	// let url = format!("https://2gis.ru/{}/search/%D0%A0%D0%B5%D1%81%D1%82%D0%BE%D1%80%D0%B0%D0%BD%D1%8B/rubricId/164?m=37.62017%2C55.753466%2F11", &city);
	driver.goto(url).await?;
	sleep(Duration::from_secs(1)).await;

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

	let number_of_elements_xpath = String::from("//span[contains(@class, \"_1xhlznaa\")]");

	let number_of_elements = match find_block(driver.clone(), number_of_elements_xpath).await {
		Ok(elem) => elem,
		Err(e) => {
			println!("error while searching name block: {}", e);
			"".to_string()
		}
	};

	let edge: i32 = (number_of_elements.parse::<f32>().unwrap_or(0.0) / 12.0).ceil() as i32;

	println!("{:?}", &edge);

	// кол-во организаций/12
	for j in 0..=edge {
		let firms_elem: Vec<WebElement> = driver.find_all(By::XPath("//body/div/div/div/div/div/div[3]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div[contains(@style, 'width: 352px')]/div[2]/div/div/div")).await?;
		println!("страница: {}", j);
		sleep(Duration::from_secs(1)).await;

		let first = firms_elem.first().unwrap();
		let last = firms_elem.last().unwrap();

		let _ = last.scroll_into_view().await?;
		sleep(Duration::from_secs(2)).await;

		let _ = first.scroll_into_view().await?;
		sleep(Duration::from_secs(1)).await;

		let mut name_xpath;

		// номер страницы после которой все упало
		if j >= 227 {
			for (i, firm_elem) in firms_elem.clone().into_iter().enumerate() {
				println!("фирма: {}", &i);

				let _ = firm_elem.scroll_into_view().await;

				if firm_elem.inner_html().await?.contains("_h2n9mw")
					|| firm_elem.inner_html().await?.contains("Нижнекамск")
					|| firm_elem.inner_html().await?.contains("Елабуга")
					|| firm_elem.inner_html().await?.contains("Альметьевск")
				{
					continue;
				}

				name_xpath = [
				"//body/div/div/div/div/div/div[3]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div[contains(@style, 'width: 352px')]/div[2]/div/div[",
				format!("{}", i + 1).as_str(),
				"]/div/div/a/span/span[1]",
				]
				.concat()
				.to_string();

				let _ = firm_elem.click().await?;
				sleep(Duration::from_secs(5)).await;

				let firm_name = match find_block(driver.clone(), name_xpath.clone()).await {
					Ok(elem) => elem,
					Err(e) => {
						println!("error while searching name block: {}", e);
						"".to_string()
					}
				};

				let url_with_coords = driver.current_url().await?;
				println!("{}", &url_with_coords);

				let mut url_parts = url_with_coords
					.path_segments()
					.unwrap()
					.collect::<Vec<&str>>();
				let firm_id;
				let coords;

				if url_parts.contains(&"branches") {
					driver.back().await?;
				}

				if j == 0 {
					firm_id = *url_parts.get_mut(6).unwrap_or(&mut "-");
					coords = *url_parts.get_mut(7).unwrap_or(&mut "-");
				} else {
					firm_id = *url_parts.get_mut(8).unwrap_or(&mut "-");
					coords = *url_parts.get_mut(9).unwrap_or(&mut "-");
				}

				dbg!(&firm_id);
				dbg!(&coords);

				// запись в бд
				let _ = sqlx::query_as!(
					TwoGisFirm,
					"INSERT INTO two_gis_firms (name, two_gis_firm_id, category_id, coords) VALUES ($1, $2, $3, $4) RETURNING *",
					&firm_name.to_string(),
					&firm_id.to_string(),
					"restaurants".to_string(),
					&coords.replace("%2C", ", ").to_string(),
				)
				.fetch_one(&data.db)
				.await;
			}
		}

		let _ = last.scroll_into_view().await?;
		sleep(Duration::from_secs(1)).await;

		let button = find_element(driver.clone(),"//body/div/div/div/div/div/div[3]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div/div[3]/div[2]/div[2]".to_string()).await?;

		// переключение пагинации
		let _ = button.click().await;
		sleep(Duration::from_secs(5)).await;
	}

	driver.quit().await?;

	Ok(())
}

pub async fn find_element(driver: WebDriver, xpath: String) -> Result<WebElement, WebDriverError> {
	let block = driver.query(By::XPath(&xpath.to_owned())).first().await?;

	Ok(block)
}

pub async fn find_block(driver: WebDriver, xpath: String) -> Result<String, WebDriverError> {
	let err_block = driver
		.query(By::XPath(&xpath.to_owned()))
		.or(By::XPath("//span[contains(@class, \"_1al0wlf\")]"))
		.first()
		.await?
		.inner_html()
		.await?;

	Ok(err_block)
}

pub async fn find_id_block(driver: WebDriver, xpath: String) -> Result<String, WebDriverError> {
	let err_block = driver
		.query(By::XPath(&xpath.to_owned()))
		.first()
		.await?
		.attr("href")
		.await?
		.unwrap_or("".to_string());

	Ok(err_block)
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
