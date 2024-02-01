use crate::models::{Counter, Firm, ReviewsCount, UpdateFirmDesc};
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

#[get("/description_processing")]
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
	let count_query_result = sqlx::query_as!(ReviewsCount, "SELECT count(*) AS count FROM firms")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		println!("Что-то пошло не так во время подсчета отзывов");
	}

	let firms_count = count_query_result.unwrap().count.unwrap();

	dbg!(&firms_count);

	let counter = sqlx::query_as!(
		Counter,
		"SELECT * FROM counter WHERE counter_id = '4bb99137-6c90-42e6-8385-83c522cde804';"
	)
	.fetch_one(&data.db)
	.await
	.unwrap();

	let start = &counter.value.clone().unwrap().parse::<i64>().unwrap();

	for j in start.clone()..firms_count {
		println!("Firm: {:?}", j + 1);

		let firm = sqlx::query_as!(
			Firm,
			"SELECT * FROM firms ORDER BY two_gis_firm_id LIMIT 1 OFFSET $1;",
			j
		)
		.fetch_one(&data.db)
		.await
		.unwrap();

		let mut firms: Vec<UpdateFirmDesc> = Vec::new();

		// ====

		let https = HttpsConnector::new();
		let client = Client::builder().build(https);
		let uri = "https://neuroapi.host/v1/chat/completions";
		let firm_name = &firm.name.clone().unwrap();
		dbg!(&firm_name);
		let preamble = format!("Проанализируй описание автосервиса {} и напиши на его основе статью-описание о том, чем занимается организация {}, какие есть преимущества сотрудничества именно с этим автосервисом {}", &firm_name, &firm_name, &firm_name);
		let oai_token = env::var("OPENAI_API_KEY").unwrap();
		let auth_header_val = format!("Bearer {}", oai_token);

		// request
		let oai_request = OAIRequest {
			model: "gpt-3.5-turbo-1106".to_string(),
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
				Firm,
				r#"UPDATE firms SET description = $1 WHERE firm_id = $2 RETURNING *"#,
				firm.description,
				firm.firm_id,
			)
			.fetch_one(&data.db)
			.await;

			dbg!(&firm);
		}

		let _ = sqlx::query_as!(
			Counter,
			r#"UPDATE counter SET value = $1 WHERE counter_id = $2 RETURNING *"#,
			(j + 1).to_string(),
			counter.counter_id,
		)
		.fetch_one(&data.db)
		.await;
	}

	Ok(())
}
