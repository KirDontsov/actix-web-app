use crate::{
	models::{LoginUserSchema, TokenClaims, User},
	AppState,
};
use actix_web::{
	cookie::{time::Duration as ActixWebDuration, Cookie, SameSite},
	post, web, HttpRequest, HttpResponse, Responder,
};
use argon2::{
	password_hash::{PasswordHash, PasswordVerifier},
	Argon2,
};
use chrono::{prelude::*, Duration};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;

#[post("/auth/login")]
async fn login_handler(
	req: HttpRequest,
	body: web::Json<LoginUserSchema>,
	data: web::Data<AppState>,
) -> impl Responder {
	let query_result = sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", body.email)
		.fetch_optional(&data.db)
		.await
		.unwrap();

	let is_valid = query_result.to_owned().map_or(false, |user| {
		if let Some(password) = &user.password {
			let parsed_hash = PasswordHash::new(password).unwrap();
			Argon2::default()
				.verify_password(body.password.as_bytes(), &parsed_hash)
				.map_or(false, |_| true)
		} else {
			false
		}
	});

	if !is_valid {
		return HttpResponse::BadRequest().json(
			json!({"status": "fail", "message": "Неправильный адрес электронной почты или пароль"}),
		);
	}

	let user = query_result.unwrap();

	let now = Utc::now();
	let iat = now.timestamp() as usize;
	let exp = (now + Duration::minutes(60)).timestamp() as usize;
	let claims: TokenClaims = TokenClaims {
		sub: user.id.to_string(),
		role: user.role.clone().unwrap_or_default(),
		exp,
		iat,
	};

	let token = encode(
		&Header::default(),
		&claims,
		&EncodingKey::from_secret(data.env.jwt_secret.as_ref()),
	)
	.unwrap();

	// In a proxy setup (nginx), when frontend is on HTTPS but backend is HTTP,
	// we need to set secure=true for the cookie to be sent from HTTPS frontend
	// The X-Forwarded-Proto header tells us the original protocol
	let is_secure = req.headers().get("x-forwarded-proto")
		.and_then(|h| h.to_str().ok())
		.map(|h| h == "https")
		.unwrap_or(false); // Default to false if header is not present

	// Determine the domain for the cookie based on the Host header
	// This ensures the cookie is set for the correct domain (e.g., sunseven.ru)
	// rather than the backend server (localhost)
	let host_header = req.headers().get("host")
		.and_then(|h| h.to_str().ok())
		.unwrap_or("localhost");
	
	// Extract just the domain part (without port)
	let domain = host_header.split(':').next().unwrap_or("localhost");

	let cookie = Cookie::build("token", token.to_owned())
		.same_site(SameSite::Lax)  // Changed from SameSite::None to SameSite::Lax for Safari compatibility
		.path("/")
		.domain(domain)  // Set cookie for the frontend domain, not the backend
		.max_age(ActixWebDuration::new(60 * 60, 0))
		.http_only(true)
		.secure(is_secure) // Set secure based on original protocol
		.finish();

	HttpResponse::Ok()
		.cookie(cookie)
		.json(json!({"status": "success", "token": token}))
}
