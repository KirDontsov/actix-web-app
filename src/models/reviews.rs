use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct Review {
	pub review_id: uuid::Uuid,
	pub firm_id: uuid::Uuid,
	pub two_gis_firm_id: Option<String>,
	pub author: Option<String>,
	pub date: Option<String>,
	pub rating: Option<String>,
	pub text: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ReviewsCount {
	pub count: Option<i64>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct SaveReview {
	pub firm_id: uuid::Uuid,
	pub two_gis_firm_id: String,
	pub author: String,
	pub date: String,
	pub text: String,
	// pub rating: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct FilteredReview {
	pub review_id: String,
	pub firm_id: String,
	pub two_gis_firm_id: Option<String>,
	pub author: Option<String>,
	pub date: Option<String>,
	pub text: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ReviewsFilterOptions {
	pub firm_id: Option<String>,
	pub page: Option<usize>,
	pub limit: Option<usize>,
}
