use crate::models::{Counter, Firm, OAIReview, Review, ReviewsCount, SaveOAIReview};
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

#[get("/reviews_processing")]
async fn reviews_processing_handler(
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

		let mut reviews: Vec<SaveOAIReview> = Vec::new();

		let reviews_by_firm = sqlx::query_as!(
			Review,
			"SELECT * FROM reviews WHERE firm_id = $1;",
			&firm.firm_id
		)
		.fetch_all(&data.db)
		.await
		.unwrap();

		if reviews_by_firm.len() < 2 {
			continue;
		}

		let reviews_string = &reviews_by_firm
			.into_iter()
			.map(|review| review.text.unwrap_or("".to_string()))
			.filter(|n| n != "")
			.collect::<Vec<String>>()
			.join("; ");

		// ====

		let https = HttpsConnector::new();
		let client = Client::builder().build(https);
		let uri = "https://neuroapi.host/v1/chat/completions";
		let firm_name = &firm.name.clone().unwrap();
		dbg!(&firm_name);
		dbg!(&reviews_string);

		let preamble = format!("Составь html страницу с текстом и списками, на основе отзывов об автосервисе {}, 
		важно, чтобы текст обязательно был оформлен html разметкой,
		важно, чтобы текст был понятен 18-летним девушкам и парням, которые не разбираются в автосервисах.
		Кратко опиши какие виды работ обсуждают люди, 
		что из этих работ было сделано хорошо, а что плохо,
		обманывают ли в этом автосервисе или нет.
		Выведи нумерованный список: плюсов и минусов если человек обратится в этот автосервис для ремонта своего автомобиля
		Важно - подсчитай и выведи не нумерованным списком сумму положительных и сумму отрицательных отзывов,
		если больше положительных отзывов, укажи что рейтинг организации хороший, 
		если примерно поровну, укажи что рейтинг организации удовлетворительный
		если больше отрицательных отзывов, укажи что рейтинг организации не удовлетворительный
		", &firm_name);
		let oai_token = env::var("OPENAI_API_KEY").unwrap();
		let auth_header_val = format!("Bearer {}", oai_token);

		// request
		let oai_request = OAIRequest {
			model: "gpt-3.5-turbo".to_string(),
			messages: vec![OAIMessage {
				role: "user".to_string(),
				content: format!("{}: {}", preamble, &reviews_string),
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

		reviews.push(SaveOAIReview {
			firm_id: firm.firm_id.clone(),
			text: json.choices[0]
				.message
				.content
				.clone()
				.replace("XYZ", &firm_name),
		});

		// запись в бд
		for review in reviews {
			let _ = sqlx::query_as!(
				OAIReview,
				r#"INSERT INTO oai_reviews (firm_id, text) VALUES ($1, $2) RETURNING *"#,
				review.firm_id,
				review.text,
			)
			.fetch_one(&data.db)
			.await;

			dbg!(&review);
		}

		let _ = sqlx::query_as!(
			Counter,
			r#"UPDATE counter SET value = $1, name = $2 WHERE counter_id = $3 RETURNING *"#,
			(j + 1).to_string(),
			counter.name,
			counter.counter_id,
		)
		.fetch_one(&data.db)
		.await;
	}

	Ok(())
}
