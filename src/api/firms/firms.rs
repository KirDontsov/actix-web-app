use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{api::CustomError, models::Firm};

impl Firm {
	pub async fn get_firm(db: &Pool<Postgres>, n: i64) -> Result<Self, CustomError> {
		let firm_query_result = sqlx::query_as!(
			Firm,
			"
			SELECT * FROM firms
			ORDER BY two_gis_firm_id LIMIT 1 OFFSET $1;
			",
			&n
		)
		.fetch_one(db)
		.await;

		if firm_query_result.is_err() {
			println!("Что-то пошло не так во время запроса get_firm");
		}

		Ok(firm_query_result.unwrap())
	}

	pub async fn get_firm_by_city_category(
		db: &Pool<Postgres>,
		table_name: String,
		city_id: Uuid,
		category_id: Uuid,
		n: i64,
	) -> Result<Self, CustomError> {
		let sql = format!(
			"
			SELECT * FROM {}
			WHERE city_id = '{}' AND category_id = '{}'
			ORDER BY two_gis_firm_id LIMIT 1 OFFSET '{}';
			",
			&table_name, &city_id, &category_id, &n,
		);
		let firm_query_result = sqlx::query_as::<_, Firm>(&sql).fetch_one(db).await;

		if firm_query_result.is_err() {
			println!("Что-то пошло не так во время запроса firm {}", &table_name);
		}

		Ok(firm_query_result.unwrap())
	}

	pub async fn get_firm_with_empty_field(
		db: &Pool<Postgres>,
		table_name: String,
		field_name: String,
		n: i64,
	) -> Result<Self, CustomError> {
		let sql = format!(
			"
			SELECT * FROM {}
			WHERE {} = ''
			ORDER BY two_gis_firm_id LIMIT 1 OFFSET '{}';
			",
			&table_name, &field_name, &n,
		);
		let firm_query_result = sqlx::query_as::<_, Firm>(&sql).fetch_one(db).await;

		if firm_query_result.is_err() {
			println!("Что-то пошло не так во время запроса firm {}", &table_name);
		}

		Ok(firm_query_result.unwrap())
	}

	pub async fn get_firm_by_url(db: &Pool<Postgres>, url: &String) -> Result<Self, CustomError> {
		let firm_query_result = sqlx::query_as!(Firm, "SELECT * FROM firms WHERE url = $1", url)
			.fetch_one(db)
			.await;

		if firm_query_result.is_err() {
			println!("Что-то пошло не так во время запроса firm");
		}

		Ok(firm_query_result.unwrap())
	}

	// TODO: доделать update_firm
	// pub async fn update_firm(db: &Pool<Postgres>, firm: SaveFirm) -> Result<Self, CustomError> {
	// 	let firm_query_result = sqlx::query_as!(
	// 		Firm,
	// 		r#"UPDATE firm SET firm_id = $1 WHERE firm_id = $2 RETURNING *"#,
	// 		(&firm.value.clone().parse::<i64>().unwrap() + 1).to_string(),
	// 		firm.firm_id,
	// 	)
	// 	.fetch_one(db)
	// 	.await;

	// 	if firm_query_result.is_err() {
	// 		println!("Что-то пошло не так во время подсчета фирм");
	// 	}

	// 	Ok(firm_query_result.unwrap())
	// }
}
