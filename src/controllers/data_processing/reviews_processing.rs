use crate::models::{Count, Firm, OAIReview, Review, SaveOAIReview};
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
#[get("/processing/reviews")]
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
	let counter_id: String = String::from("a518df5b-1258-482b-aa57-e07c57961a69");
	let https = HttpsConnector::new();
	let client = Client::builder().build(https);
	let uri = std::env::var("OPENAI_API_BASE").unwrap();
	let oai_token = env::var("OPENAI_API_KEY").unwrap();
	let model = "gpt-3.5-turbo".to_string();
	let auth_header_val = format!("Bearer {}", oai_token);
	let table = String::from("firms");

	let firms_count = Count::count(&data.db, table).await.unwrap_or(0);
	dbg!(&firms_count);

	// получаем из базы начало счетчика
	let start = get_counter(&data.db, &counter_id).await;

	for j in start.clone()..firms_count {
		println!("Firm: {:?}", j + 1);

		let firm = Firm::get_firm(&data.db, j).await?;

		let firm_id = &firm.firm_id.clone();
		let firm_name = &firm.name.clone().unwrap();
		dbg!(&firm_id);
		dbg!(&firm_name);

		let oai_review = sqlx::query_as!(
			OAIReview,
			r#"SELECT * FROM oai_reviews_copy WHERE firm_id = $1;"#,
			&firm.firm_id
		)
		.fetch_one(&data.db)
		.await;

		if oai_review.is_ok() {
			println!("Already exists");
			continue;
		}

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
			println!("SKIP - Too few reviews");
			continue;
		}

		let reviews_string = &reviews_by_firm
			.into_iter()
			.map(|review| review.text.unwrap_or("".to_string()))
			.filter(|n| n != "")
			.collect::<Vec<String>>()
			.join("; ");

		// system preamble
		let first_message = OAIMessage {
			role: "system".to_string(),
			content: "Ты писатель-копирайтер, пишешь SEO оптимизированные тексты".to_string(),
		};

		// user preamble
		let preamble = format!("
		Вот отзывы которые ты должен проанализировать: {}

		Напиши большую статью, на основе этих отзывов об автосервисе {}, 
		важно, чтобы текст был понятен 18-летним девушкам и парням, которые не разбираются в автосервисах, но без упоминания слова - Статья

		Подробно опиши в этой статье: какие виды работ обсуждают люди, 
		что из этих работ было сделано хорошо, а что плохо,
		обманывают ли в этом автосервисе или нет.
		Например, если об этом говорят в отзывах:
		В отзывах обсуждаются следующие услуги: 
		1. Кузовной ремонт - плохое качество
		2. Мастера - отзывчивые

		Выведи нумерованный список: плюсов и минусов если человек обратится в этот автосервис для ремонта своего автомобиля.
		Например, если об этом говорят в отзывах: 
		Плюсы
		1. Хорошо чинят машины
		2. Хорошо красят
		Минусы
		1. Далеко от центра города

		Важно - подсчитай и выведи не нумерованным списком сумму положительных и сумму отрицательных отзывов,
		Например: 
		Положительных отзывов - 15
		Отрицательных отзывов - 5

		Сделай выводы, на основе плюсов и минусов организации, количества положительных и отрицательных отзывов.
		Например:
		У организации больше положительных отзывов, укажи что рейтинг организации хороший, и объясни почему.
		Или например:
		У организации поровну положительных и отрицательных отзывов, укажи что рейтинг организации удовлетворительный, и объясни почему.
		Или например:
		У организации больше отрицательных отзывов, укажи что рейтинг организации не удовлетворительный, и объясни почему.
		
		В конце текста укажи: *Пожалуйста, обратите внимание, что данный обзор сформирован нейросетью и может быть не точным*
		", &reviews_string.chars().take(3800).collect::<String>(), &firm_name);

		let second_message = OAIMessage {
			role: "user".to_string(),
			content: format!("{}", preamble),
		};

		// request
		let oai_request = OAIRequest {
			model: model.clone(),
			messages: vec![first_message, second_message],
		};

		let body = Body::from(serde_json::to_vec(&oai_request)?);
		let req = Request::post(&uri)
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
			let x = sqlx::query_as!(
				OAIReview,
				r#"INSERT INTO oai_reviews_copy (firm_id, text) VALUES ($1, $2) RETURNING *"#,
				review.firm_id,
				review.text,
			)
			.fetch_one(&data.db)
			.await;

			dbg!(&x);
		}

		let _ = update_counter(&data.db, &counter_id, &(j + 1).to_string()).await;
	}

	Ok(())
}
