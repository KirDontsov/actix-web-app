use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct Category {
	pub category_id: uuid::Uuid,
	pub name: Option<String>,
	pub abbreviation: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CategoriesCount {
	pub count: Option<i64>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct SaveCategory {
	pub name: String,
	pub abbreviation: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct FilteredCategory {
	pub category_id: String,
	pub name: Option<String>,
	pub abbreviation: Option<String>,
}
