use serde::Deserialize;
use sqlx::FromRow;

#[derive(Deserialize, Debug)]
pub struct FilterOptions {
	pub page: Option<usize>,
	pub limit: Option<usize>,
}

#[derive(Deserialize, Debug, FromRow)]
pub struct Count {
	pub count: Option<i64>,
}
