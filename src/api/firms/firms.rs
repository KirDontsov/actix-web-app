use sqlx::{Pool, Postgres};

use crate::{
	api::CustomError,
	models::{Firm, FirmsCount},
};

impl Firm {
	pub async fn get_firm(db: &Pool<Postgres>, n: i64) -> Result<Self, CustomError> {
		let firm_query_result = sqlx::query_as!(
			Firm,
			"SELECT * FROM firms ORDER BY two_gis_firm_id LIMIT 1 OFFSET $1;",
			&n
		)
		.fetch_one(db)
		.await;

		if firm_query_result.is_err() {
			println!("Что-то пошло не так во время запроса get_firm");
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

impl FirmsCount {
	pub async fn count_firm(db: &Pool<Postgres>) -> Result<i64, CustomError> {
		let count_query_result = sqlx::query_as!(FirmsCount, "SELECT count(*) AS count FROM firms")
			.fetch_one(db)
			.await;

		if count_query_result.is_err() {
			println!("Что-то пошло не так во время запроса count_firm");
		}

		let result = count_query_result.unwrap().count.unwrap();

		println!("{:?}", &result);

		Ok(result)
	}
}
