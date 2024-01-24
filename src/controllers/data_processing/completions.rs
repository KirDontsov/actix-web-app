use crate::models::{Firm, Review, ReviewsCount, SaveReview};
use crate::AppState;
use actix_web::{get, web, HttpResponse, Responder};
use hyper::body::Buf;
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

#[get("/completions")]
async fn completions_handler(
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	let _ = processing(data).await;

	let json_response = serde_json::json!({
		"status":  "success",
	});

	HttpResponse::Ok().json(json_response)
}

async fn processing(data: web::Data<AppState>) -> Result<(), Box<dyn std::error::Error>> {
	let count_query_result = sqlx::query_as!(ReviewsCount, "SELECT count(*) AS count FROM reviews")
		.fetch_one(&data.db)
		.await;

	if count_query_result.is_err() {
		println!("Что-то пошло не так во время подсчета отзывов");
	}

	let firms_count = count_query_result.unwrap().count.unwrap();

	dbg!(&firms_count);

	for i in 0..5 {
		println!("Firm: {:?}", i);

		let firm = sqlx::query_as!(
			Firm,
			"SELECT * FROM firms ORDER BY two_gis_firm_id LIMIT 1 OFFSET $1;",
			i
		)
		.fetch_one(&data.db)
		.await
		.unwrap();

		// let mut reviews: Vec<SaveReview> = Vec::new();

		let reviews_by_firm = sqlx::query_as!(
			Review,
			"SELECT * FROM reviews WHERE firm_id = $1;",
			&firm.firm_id
		)
		.fetch_all(&data.db)
		.await
		.unwrap();

		let reviews = &reviews_by_firm
			.into_iter()
			.map(|review| review.text.unwrap_or("".to_string()))
			.filter(|n| n != "")
			.collect::<Vec<String>>()
			.join("; ");

		dbg!(&reviews);

		// ====

		let https = HttpsConnector::new();
		let client = Client::builder().build(https);
		let uri = "https://api.openai.com/v1/chat/completions";
		let preamble = "Проанализируй и дай краткое содержание отзывов об организации, объясни какие есть минусы и какие плюсы в этой организации";
		let oai_token = env::var("OPENAI_API_KEY").unwrap();
		let auth_header_val = format!("Bearer {}", oai_token);

		// request
		let oai_request = OAIRequest {
			model: "gpt-3.5-turbo".to_string(),
			messages: vec![OAIMessage {
				role: "user".to_string(),
				content: format!("{}: {}", preamble, &reviews),
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

		println!(" === {}", json.choices[0].message.content);
	}

	Ok(())
}
