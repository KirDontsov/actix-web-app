use actix_web::web;

use crate::controllers::auth::*;
use crate::controllers::routes::*;
use crate::controllers::user::*;

pub fn config(conf: &mut web::ServiceConfig) {
	let scope = web::scope("/api")
		.service(register_handler)
		.service(login_handler)
		.service(logout_handler)
		.service(parser_handler)
		.service(get_user_handler)
		.service(get_users_handler)
		.service(update_user_handler)
		.service(get_me_handler);

	conf.service(scope);
}
