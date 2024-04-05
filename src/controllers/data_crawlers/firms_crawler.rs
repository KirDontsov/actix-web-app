use crate::{api::Driver, jwt_auth, models::TwoGisFirm, AppState};
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

	// автосервисы
	let url = format!("https://2gis.ru/{}/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81/rubricId/9041/37.716023%2C55.779159?m=37.62017%2C55.753466%2F11", &city);
	// рестораны
	// let url = format!("https://2gis.ru/{}/search/%D0%A0%D0%B5%D1%81%D1%82%D0%BE%D1%80%D0%B0%D0%BD%D1%8B/rubricId/164?m=37.62017%2C55.753466%2F11", &city);
	driver.goto(url).await?;

	sleep(Duration::from_secs(1)).await;

	// кол-во организаций/13
	for j in 0..255 {
		let firms_elem: Vec<WebElement> = driver.find_all(By::XPath("//body/div/div/div/div/div/div[2]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div[contains(@style, 'width: 352px')]/div[2]/div/div")).await?;
		let last = firms_elem.last().unwrap();
		last.scroll_into_view().await?;
		println!("страница: {}", j);
		sleep(Duration::from_secs(1)).await;

		let mut firms = Vec::new();

		let mut name_xpath;
		let mut firm_id_xpath;

		for (i, firm_elem) in firms_elem.clone().into_iter().enumerate() {
			if i == 2 {
				continue;
			}
			name_xpath = [
			"//body/div/div/div/div/div/div[2]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div[contains(@style, 'width: 352px')]/div[2]/div/div[",
			format!("{}", i + 1).as_str(),
			"]/div/div/a/span/span[1]",
			]
			.concat()
			.to_string();

			firm_id_xpath = [
			"//body/div/div/div/div/div/div[2]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div[contains(@style, 'width: 352px')]/div[2]/div/div[",
			format!("{}", i + 1).as_str(),
			"]/div/div/a",
			]
			.concat()
			.to_string();

			let firm_name = match find_block(driver.clone(), name_xpath).await {
				Ok(elem) => elem,
				Err(e) => {
					println!("error while searching name block: {}", e);
					"".to_string()
				}
			};

			let firm_id = match find_id_block(driver.clone(), firm_id_xpath).await {
				Ok(elem) => elem,
				Err(e) => {
					println!("error while searching id block: {}", e);
					"".to_string()
				}
			};

			let split_target = format!("/{}/firm/", &city);

			// TODO: попробовать заменить на regexp
			let url_part_one = *firm_id
				.split(&split_target)
				.collect::<Vec<&str>>()
				.get_mut(1)
				.unwrap_or(&mut "-?");

			let res = *url_part_one
				.split("?")
				.collect::<Vec<&str>>()
				.get(0)
				.unwrap_or(&mut "");

			let firm: (String, String) = (firm_name, res.to_string());
			firms.push(firm);
		}

		// запись в бд
		for firm in firms {
			let _ = sqlx::query_as!(
				TwoGisFirm,
				"INSERT INTO two_gis_firms (name, two_gis_firm_id, category_id) VALUES ($1, $2, $3) RETURNING *",
				firm.0.to_string(),
				firm.1.to_string(),
				"car_service".to_string(),
			)
			.fetch_one(&data.db)
			.await;

			println!("{} -- {}", firm.0, firm.1)
		}

		let button_elems: Vec<WebElement> = driver.find_all(By::XPath("//body/div/div/div/div/div/div[2]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div/div[3]/div[2]/div[2]")).await?;

		// переключение пагинации
		for button_elem in button_elems {
			button_elem.click().await?;
			sleep(Duration::from_secs(5)).await;
		}
	}

	driver.quit().await?;

	Ok(())
}

pub async fn find_block(driver: WebDriver, xpath: String) -> Result<String, WebDriverError> {
	let err_block = driver
		.query(By::XPath(&xpath.to_owned()))
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
