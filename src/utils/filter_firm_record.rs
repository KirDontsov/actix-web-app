use crate::models::{
	ExtFilteredFirmWithOaiDescription, ExtFirmWithOaiDescription, FilteredFirm, Firm,
};

pub fn filter_ext_firm_record(
	firm: &ExtFirmWithOaiDescription,
) -> ExtFilteredFirmWithOaiDescription {
	ExtFilteredFirmWithOaiDescription {
		firm_id: firm.firm_id.to_string(),
		name: firm.name.to_owned(),
		address: firm.address.to_owned(),
		site: firm.site.to_owned(),
		default_phone: firm.default_phone.to_owned(),
		oai_description_value: firm.oai_description_value.to_owned(),
		description: firm.description.to_owned(),
	}
}
