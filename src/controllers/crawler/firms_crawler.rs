use crate::{jwt_auth, models::Firm, AppState};
use actix_web::{get, web, HttpResponse, Responder};
use thirtyfour::prelude::*;
use tokio::time::Duration;

#[get("/crawler/firms")]
async fn firms_crawler_handler(
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

	driver.goto("https://2gis.ru/spb/search/%D0%B0%D0%B2%D1%82%D0%BE%D1%81%D0%B5%D1%80%D0%B2%D0%B8%D1%81?m=30.385039%2C59.980836%2F16.24").await?;

	tokio::time::sleep(Duration::from_secs(5)).await;

	// кол-во организаций/13
	for j in 0..255 {
		let firms_elem: Vec<WebElement> = driver.find_all(By::XPath("//body/div/div/div/div/div/div[2]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div[contains(@style, 'width: 352px')]/div[2]/div/div")).await?;
		let last = firms_elem.last().unwrap();
		last.scroll_into_view().await?;
		tokio::time::sleep(Duration::from_secs(5)).await;

		let mut firms = Vec::new();

		let mut name_xpath;
		let mut firm_id_xpath;

		for (i, firm_elem) in firms_elem.clone().into_iter().enumerate() {
			if i > 0 && j != 0 {
				name_xpath = [
			"//body/div/div/div/div/div/div[2]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div[contains(@style, 'width: 352px')]/div[2]/div/div[",
			format!("{}", i).as_str(),
			"]/div/div/a/span/span[1]",
			]
			.concat()
			.to_string();

				firm_id_xpath = [
			"//body/div/div/div/div/div/div[2]/div/div/div[2]/div/div/div/div[2]/div[2]/div/div/div/div[contains(@style, 'width: 352px')]/div[2]/div/div[",
			format!("{}", i).as_str(),
			"]/div/div/a",
			]
			.concat()
			.to_string();

				let firm_name = firm_elem
					.find(By::XPath(&name_xpath))
					.await?
					.inner_html()
					.await?;

				let Some(firm_id) = firm_elem
					.find(By::XPath(&firm_id_xpath))
					.await?
					.attr("href")
					.await?
				else {
					panic!("no href!");
				};

				// TODO: попробовать заменить на regexp
				let url_part_one = firm_id.split("/spb/firm/").collect::<Vec<&str>>()[1];
				let res = &url_part_one.split("?").collect::<Vec<&str>>()[0];

				let firm = (firm_name, res.to_string());
				firms.push(firm);
			}
		}

		// запись в бд
		for firm in firms {
			let _ = sqlx::query_as!(
				Firm,
				"INSERT INTO firms (name, firm_id, category_id) VALUES ($1, $2, $3) RETURNING *",
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
			tokio::time::sleep(Duration::from_secs(5)).await;
		}
		println!("страница: {}", j);
	}

	driver.quit().await?;

	Ok(())
}
