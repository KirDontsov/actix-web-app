use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct Firm {
	pub id: uuid::Uuid,
	pub name: Option<String>,
	pub firm_id: Option<String>,
	pub category_id: Option<String>,
	#[serde(rename = "createdAt")]
	pub created_at: Option<DateTime<Utc>>,
	#[serde(rename = "updatedAt")]
	pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Debug)]
pub struct FirmsCount {
	pub count: Option<i64>,
}
