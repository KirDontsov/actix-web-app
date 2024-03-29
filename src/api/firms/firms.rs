use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{api::CustomError, models::Firm};

impl Firm {
	pub async fn get_firm(db: &Pool<Postgres>, n: i64) -> Result<Self, CustomError> {
		let firm_query_result = sqlx::query_as!(
			Firm,
			"
			SELECT * FROM firms
			WHERE city_id = '566e11b5-79f5-4606-8c18-054778f3daf6'
			AND category_id = '3ebc7206-6fed-4ea7-a000-27a74e867c9a'
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
			println!("Что-то пошло не так во время запроса count {}", &table_name);
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
