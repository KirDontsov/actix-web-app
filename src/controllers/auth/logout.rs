use crate::jwt_auth;
use actix_web::{
	cookie::{time::Duration as ActixWebDuration, Cookie, SameSite},
	get, HttpResponse, Responder,
};
use serde_json::json;

#[get("/auth/logout")]
async fn logout_handler(_: jwt_auth::JwtMiddleware) -> impl Responder {
	let cookie = Cookie::build("token", "")
		.path("/")
		.max_age(ActixWebDuration::new(-1, 0))
		.http_only(true)
		.secure(false)  // Using false for logout to ensure it works in all browsers
		.same_site(SameSite::Lax)  // Added SameSite for consistency
		.finish();

	HttpResponse::Ok()
		.cookie(cookie)
		.json(json!({"status": "success"}))
}
