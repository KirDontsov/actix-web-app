use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct CitiesCount {
	pub count: Option<i64>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct City {
	pub city_id: uuid::Uuid,
	pub name: Option<String>,
	pub abbreviation: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct SaveCity {
	pub name: String,
	pub abbreviation: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct FilteredCity {
	pub city_id: String,
	pub name: Option<String>,
	pub abbreviation: Option<String>,
}
