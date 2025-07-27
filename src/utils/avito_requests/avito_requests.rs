use crate::models::{AvitoRequest, FilteredAvitoRequest};

pub fn filter_add_avito_request_record(avito_request: &AvitoRequest) -> FilteredAvitoRequest {
	FilteredAvitoRequest {
		request_id: avito_request.request_id.to_string(),
		user_id: avito_request.user_id.to_string(),
		request: avito_request.request.to_owned(),
		city: avito_request.city.to_owned(),
		coords: avito_request.coords.to_owned(),
		radius: avito_request.radius.to_owned(),
		district: avito_request.district.to_owned(),
	}
}
