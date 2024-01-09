use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct Category {
	pub category_id: uuid::Uuid,
	pub name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CategoriesCount {
	pub count: Option<i64>,
}
