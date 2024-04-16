use actix_web::web;

use crate::controllers::auth::*;
use crate::controllers::categories::*;
use crate::controllers::cities::*;
use crate::controllers::data_crawlers::*;
use crate::controllers::data_processing::*;
use crate::controllers::firms::*;
use crate::controllers::images::*;
use crate::controllers::oai_reviews::*;
use crate::controllers::oai_descriptions::*;
use crate::controllers::prices::*;
use crate::controllers::quotes::*;
use crate::controllers::reviews::*;
use crate::controllers::routes::*;
use crate::controllers::types::*;
use crate::controllers::user::*;

pub fn config(conf: &mut web::ServiceConfig) {
	let scope = web::scope("/api")
		// auth
		.service(register_handler)
		.service(login_handler)
		.service(get_me_handler)
		.service(logout_handler)
		// parsers
		// .service(firms_crawler_handler)
		// .service(firms_info_crawler_handler)
		// .service(firms_reviews_crawler_handler)
		// .service(firms_description_crawler_handler)
		// .service(firms_images_crawler_handler)
		// .service(firms_prices_crawler_handler)
		// processing
		// .service(images_processing_handler)
		// .service(reviews_processing_handler)
		// .service(description_processing_handler)
		//user
		.service(get_users_handler)
		.service(get_user_handler)
		.service(update_user_handler)
		// quote
		.service(get_quotes_handler)
		.service(get_quote_handler)
		.service(add_quote_handler)
		// firm
		.service(get_firms_handler)
		.service(get_firm_handler)
		// cities
		.service(get_city_handler)
		.service(get_cities_handler)
		// categories
		.service(get_category_handler)
		.service(get_categories_handler)
		// types
		.service(get_types_handler)
		// reviews
		.service(get_reviews_handler)
		.service(add_review_handler)
		.service(get_oai_reviews_handler)
		// description
		.service(get_oai_description_by_firm_handler)
		// images
		.service(get_images_handler)
		.service(mir_far_crawler_handler)
		// prices
		.service(get_prices_handler);

	conf.service(scope);
}
