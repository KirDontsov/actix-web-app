use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct AvitoRequest {
	pub request_id: uuid::Uuid,
	pub user_id: uuid::Uuid,
	pub request: Option<String>,
	pub city: Option<String>,
	pub coords: Option<String>,
	pub radius: Option<String>,
	pub district: Option<String>,
	#[serde(rename = "createdTs")]
	pub created_ts: Option<DateTime<Utc>>,
	#[serde(rename = "updatedTs")]
	pub updated_ts: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct SaveAvitoRequest {
	pub request: String,
	pub city: String,
	pub coords: String,
	pub radius: String,
	pub district: String,
}

#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct FilteredAvitoRequest {
	pub request_id: String,
	pub user_id: String,
	pub request: Option<String>,
	pub city: Option<String>,
	pub coords: Option<String>,
	pub radius: Option<String>,
	pub district: Option<String>,
}
