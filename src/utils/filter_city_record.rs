use crate::models::{City, FilteredCity};

pub fn filter_city_record(city: &City) -> FilteredCity {
	FilteredCity {
		city_id: city.city_id.to_string(),
		name: city.name.to_owned(),
		abbreviation: city.abbreviation.to_owned(),
	}
}
