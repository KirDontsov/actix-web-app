use crate::jwt_auth;
use actix_web::{
	cookie::{time::Duration as ActixWebDuration, Cookie, SameSite},
	get, HttpRequest, HttpResponse, Responder,
};
use serde_json::json;

#[get("/auth/logout")]
async fn logout_handler(
	req: HttpRequest,
	_: jwt_auth::JwtMiddleware,
) -> impl Responder {
	// Determine if the request came through HTTPS by checking various headers
	// This handles cases where the app is behind a proxy/load balancer
	let is_secure = req.headers().get("x-forwarded-proto")
		.and_then(|h| h.to_str().ok())
		.map(|h| h == "https")
		.unwrap_or_else(|| {
			// If x-forwarded-proto is not present, check other common headers
			req.headers().get("x-forwarded-protocol")
				.and_then(|h| h.to_str().ok())
				.map(|h| h == "https")
				.unwrap_or_else(|| {
					req.headers().get("x-url-scheme")
						.and_then(|h| h.to_str().ok())
						.map(|h| h == "https")
						.unwrap_or(false) // For logout, default to false if uncertain
				})
		});

	let cookie = Cookie::build("token", "")
		.path("/")
		.max_age(ActixWebDuration::new(-1, 0))  // Expire the cookie
		.http_only(true)
		.secure(is_secure) // Use dynamically determined secure flag to match how cookie was set
		.same_site(SameSite::Lax)  // Added SameSite for consistency
		.finish();

	HttpResponse::Ok()
	.cookie(cookie)
	.json(json!({"status": "success"}))
}
