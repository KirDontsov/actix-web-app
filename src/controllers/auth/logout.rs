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
	// In a proxy setup (nginx), when frontend is on HTTPS but backend is HTTP,
	// we need to match the secure setting of the original cookie
	// The X-Forwarded-Proto header tells us the original protocol
	let is_secure = req.headers().get("x-forwarded-proto")
		.and_then(|h| h.to_str().ok())
		.map(|h| h == "https")
		.unwrap_or(false); // Default to false if header is not present

	let cookie = Cookie::build("token", "")
		.path("/")
		.max_age(ActixWebDuration::new(-1, 0)) // Expire the cookie
		.http_only(true)
		.secure(is_secure) // Match the secure setting to properly clear the cookie
		.same_site(SameSite::None) // Added SameSite for consistency
		.finish();

	HttpResponse::Ok()
		.cookie(cookie)
		.json(json!({"status": "success"}))
}
