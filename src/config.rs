#[derive(Debug, Clone)]
pub struct Config {
	pub database_url: String,
	pub jwt_secret: String,
	pub rabbitmq_url: String,
	pub cookie_secure: bool,
}

impl Config {
	pub fn init() -> Config {
		let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
		let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
		let rabbitmq_url = std::env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set");
		let cookie_secure = std::env::var("COOKIE_SECURE")
			.unwrap_or_else(|_| "true".to_string())
			.parse()
			.expect("COOKIE_SECURE must be a boolean value (true/false)");
		Config {
			database_url,
			jwt_secret,
			rabbitmq_url,
			cookie_secure,
		}
	}
}
