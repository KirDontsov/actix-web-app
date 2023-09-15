use actix_web::web;

use crate::handlers::auth::login_user_handler;
use crate::handlers::auth::logout_user_handler;
use crate::handlers::auth::register_user_handler;
use crate::handlers::routes::get_me_handler;

pub fn config(conf: &mut web::ServiceConfig) {
	let scope = web::scope("/api")
		.service(register_user_handler)
		.service(login_user_handler)
		.service(logout_user_handler)
		.service(get_me_handler);

	conf.service(scope);
}
