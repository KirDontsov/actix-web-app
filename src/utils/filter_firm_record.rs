use crate::models::{FilteredFirm, Firm};

pub fn filter_firm_record(firm: &Firm) -> FilteredFirm {
	FilteredFirm {
		firm_id: firm.firm_id.to_string(),
		two_gis_firm_id: firm.two_gis_firm_id.to_owned(),
		category_id: firm.category_id.to_string(),
		name: firm.name.to_owned(),
		address: firm.address.to_owned(),
		site: firm.site.to_owned(),
		default_phone: firm.default_phone.to_owned(),
	}
}
