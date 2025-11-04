#[derive(Debug, Clone)]
pub struct Config {
	pub database_url: String,
	pub jwt_secret: String,
	pub jwt_expires_in: String,
	pub jwt_maxage: i32,
	pub rabbitmq_username: String,
	pub rabbitmq_password: String,
	pub rabbitmq_host: String,
	pub rabbitmq_port: String,
}

impl Config {
	pub fn init() -> Config {
		let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
		let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
		let jwt_expires_in = std::env::var("JWT_EXPIRED_IN").expect("JWT_EXPIRED_IN must be set");
		let jwt_maxage = std::env::var("JWT_MAXAGE").expect("JWT_MAXAGE must be set");
		let rabbitmq_username =
			std::env::var("RABBITMQ_USERNAME").expect("RABBITMQ_USERNAME must be set");
		let rabbitmq_password =
			std::env::var("RABBITMQ_PASSWORD").expect("RABBITMQ_PASSWORD must be set");
		let rabbitmq_host = std::env::var("RABBITMQ_HOST").expect("RABBITMQ_HOST must be set");
		let rabbitmq_port = std::env::var("RABBITMQ_PORT").expect("RABBITMQ_PORT must be set");
		Config {
			database_url,
			jwt_secret,
			jwt_expires_in,
			jwt_maxage: jwt_maxage.parse::<i32>().unwrap(),
			rabbitmq_username,
			rabbitmq_password,
			rabbitmq_host,
			rabbitmq_port,
		}
	}
}
