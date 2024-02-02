use crate::models::{Counter, SaveCounter};
use sqlx::{Pool, Postgres};

pub async fn get_counter(db: Pool<Postgres>, id: &String) -> i64 {
	let counter_query_result = Counter::get_counter(db.clone(), id).await;
	if counter_query_result.is_err() {
		println!("Что-то пошло не так во время подсчета фирм");
	}
	counter_query_result
		.unwrap()
		.value
		.clone()
		.unwrap()
		.parse::<i64>()
		.unwrap()
}

pub async fn update_counter(db: Pool<Postgres>, id: &String, value: &String) {
	let _ = Counter::update_counter(
		db.clone(),
		SaveCounter {
			counter_id: uuid::Uuid::parse_str(&id).unwrap(),
			value: value.clone(),
		},
	);
}
