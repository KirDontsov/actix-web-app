use actix_web::web;

use crate::controllers::auth::*;
use crate::controllers::crawler::*;
use crate::controllers::quote::*;
use crate::controllers::routes::*;
use crate::controllers::user::*;

pub fn config(conf: &mut web::ServiceConfig) {
	let scope = web::scope("/api")
		.service(register_handler)
		.service(login_handler)
		.service(logout_handler)
		.service(firms_crawler_handler)
		.service(firms_info_crawler_handler)
		.service(firms_reviews_crawler_handler)
		.service(get_user_handler)
		.service(get_users_handler)
		.service(update_user_handler)
		.service(get_quote_handler)
		.service(get_quotes_handler)
		.service(add_quote_handler)
		.service(get_me_handler);

	conf.service(scope);
}
