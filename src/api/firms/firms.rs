use sqlx::{Pool, Postgres};

use crate::{api::CustomError, models::Firm};

impl Firm {
	pub async fn get_firm(db: &Pool<Postgres>, n: i64) -> Result<Self, CustomError> {
		let firm_query_result = sqlx::query_as!(
			Firm,
			"
			SELECT * FROM firms
			WHERE category_id = '565ad1cb-b891-4185-ac75-24ab3898cf22'
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
