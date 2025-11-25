#[derive(Debug, Clone)]
pub struct Config {
	pub database_url: String,
	pub jwt_secret: String,
	pub rabbitmq_url: String,
}

impl Config {
	pub fn init() -> Config {
		let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
		let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
		let rabbitmq_url = std::env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set");

		Config {
			database_url,
			jwt_secret,
			rabbitmq_url,
		}
	}
}
