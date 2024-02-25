use crate::models::{Firm, OAIDescription, ReviewsCount, UpdateFirmDesc};
use crate::utils::{get_counter, update_counter};
use crate::AppState;
use actix_web::web::Buf;
use actix_web::{get, web, HttpResponse, Responder};
use hyper::{header, Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Deserialize, Debug)]
struct OAIMessage {
	role: String,
	content: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct OAIResponse {
	id: Option<String>,
	object: Option<String>,
	created: Option<u64>,
	model: Option<String>,
	choices: Vec<OAIChoices>,
}

// for choices
#[derive(Deserialize, Serialize, Debug)]
struct OAIChoices {
	index: u8,
	logprobs: Option<u8>,
	finish_reason: String,
	message: OAIMessage,
}

#[derive(Serialize, Debug)]
struct OAIRequest {
	model: String,
	messages: Vec<OAIMessage>,
}

#[allow(unreachable_code)]
#[get("/processing/description")]
async fn description_processing_handler(
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	loop {
		let mut needs_to_restart = true;
		if needs_to_restart {
			let _: Result<(), Box<dyn std::error::Error>> = match processing(data.clone()).await {
				Ok(x) => {
					needs_to_restart = false;
					Ok(x)
				}
				Err(e) => {
					println!("{:?}", e);
					needs_to_restart = true;
					Err(e)
				}
			};
		}
	}
	let json_response = serde_json::json!({
		"status":  "success",
	});
	HttpResponse::Ok().json(json_response)
}

async fn processing(data: web::Data<AppState>) -> Result<(), Box<dyn std::error::Error>> {
	let counter_id: String = String::from("5e4f8432-c1db-4980-9b63-127fd320cdde");
	let oai_token = env::var("OPENAI_API_KEY").unwrap();
	let auth_header_val = format!("Bearer {}", oai_token);
	let count_query_result = sqlx::query_as!(ReviewsCount, "SELECT count(*) AS count FROM firms")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		println!("Что-то пошло не так во время подсчета отзывов");
	}

	let firms_count = count_query_result.unwrap().count.unwrap();

	dbg!(&firms_count);

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;

	for j in start.clone()..firms_count {
		println!("Firm: {:?}", j + 1);

		let firm = Firm::get_firm(&data.db, j).await?;

		let mut firms: Vec<UpdateFirmDesc> = Vec::new();

		// ====

		let https = HttpsConnector::new();
		let client = Client::builder().build(https);
		let uri = "https://neuroapi.host/v1/chat/completions";

		let firm_id = &firm.firm_id.clone();
		let firm_name = &firm.name.clone().unwrap();
		let firm_desc = &firm.description.clone().unwrap();
		let firm_phone = &firm.default_phone.clone().unwrap();
		dbg!(&firm_id);
		dbg!(&firm_name);

		let oai_description = sqlx::query_as!(
			OAIDescription,
			r#"SELECT * FROM oai_descriptions WHERE firm_id = $1;"#,
			&firm.firm_id
		)
		.fetch_one(&data.db)
		.await;

		if oai_description.is_ok() {
			println!("Already exists");
			continue;
		}
		let preamble = format!("Вот описание автосервиса которое ты должен проанализировать: {}

		Напиши большую статью об автосервисе, на основе анализа этого описания {}, 
		важно, чтобы текст был понятен 18-летним девушкам и парням, которые не разбираются в автосервисах, но без упоминания слова - \"Статья\"

		Подробно опиши в этой статье: какие виды работ может осуществлять данная организация, например, если об этом указано в описании:
		Данная организация может оказывать следующие виды работ: 
		1. Кузовной ремонт

		Придумай в чем заключается миссия данной организации по ремонту автомобилей, чем она помогает людям.

		Укажи что в компании работают опытные и квалифицированные сотрудники, которые всегда помогут и сделают это быстро и качественно.
		
		В конце текста укажи: Для получения более детальной информации позвоните по номеру: {}
		
		И перечисли все виды работ, которые могут быть свзаны с ремонтом автомобиля
		", &firm_desc, &firm_name, &firm_phone);

		// request
		let oai_request = OAIRequest {
			model: "gpt-3.5-turbo".to_string(),
			messages: vec![OAIMessage {
				role: "user".to_string(),
				content: format!(
					"{}: {}",
					preamble,
					&firm
						.description
						.clone()
						.unwrap_or("Отсутствует описание".to_string())
				),
			}],
		};

		let body = Body::from(serde_json::to_vec(&oai_request)?);
		let req = Request::post(uri)
			.header(header::CONTENT_TYPE, "application/json")
			.header("Authorization", &auth_header_val)
			.body(body)
			.unwrap();

		// response
		let res = client.request(req).await?;
		let body = hyper::body::aggregate(res).await?;
		let json: OAIResponse = serde_json::from_reader(body.reader())?;

		firms.push(UpdateFirmDesc {
			firm_id: firm.firm_id.clone(),
			description: json.choices[0]
				.message
				.content
				.clone()
				.replace("XYZ", &firm_name),
		});

		// запись в бд
		for firm in firms {
			let _ = sqlx::query_as!(
				OAIDescription,
				r#"INSERT INTO oai_descriptions (firm_id, oai_description_value) VALUES ($1, $2) RETURNING *"#,
				firm.firm_id,
				firm.description,
			)
			.fetch_one(&data.db)
			.await;

			dbg!(&firm);
		}

		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
	}

	Ok(())
}
