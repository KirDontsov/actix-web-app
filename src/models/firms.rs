use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct TwoGisFirm {
	pub firm_id: Uuid,
	pub name: Option<String>,
	pub two_gis_firm_id: Option<String>,
	pub category_id: Option<String>,
	#[serde(rename = "createdTs")]
	pub created_ts: Option<DateTime<Utc>>,
	#[serde(rename = "updatedTs")]
	pub updated_ts: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Debug)]
pub struct FirmsCount {
	pub count: Option<i64>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone, Default)]
pub struct Firm {
	pub firm_id: Uuid,
	pub two_gis_firm_id: Option<String>,
	pub category_id: Uuid,
	pub type_id: Uuid,
	pub name: Option<String>,
	pub description: Option<String>,
	pub address: Option<String>,
	pub floor: Option<String>,
	pub site: Option<String>,
	pub default_email: Option<String>,
	pub default_phone: Option<String>,
	#[serde(rename = "createdTs")]
	pub created_ts: Option<DateTime<Utc>>,
	#[serde(rename = "updatedTs")]
	pub updated_ts: Option<DateTime<Utc>>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct SaveFirm {
	pub two_gis_firm_id: String,
	pub category_id: Uuid,
	pub type_id: Uuid,
	pub name: String,
	pub address: String,
	// pub floor: String,
	pub default_phone: String,
	pub site: String,
	// pub default_email: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct FilteredFirm {
	pub firm_id: String,
	pub two_gis_firm_id: Option<String>,
	pub category_id: String,
	pub name: Option<String>,
	pub address: Option<String>,
	pub site: Option<String>,
	pub default_phone: Option<String>,
}
