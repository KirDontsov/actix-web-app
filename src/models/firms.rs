use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::TsVector;
use uuid::Uuid;
use std::error::Error;
use sqlx::{decode::Decode, postgres::PgValueRef, types::Type, Postgres};

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct TwoGisFirm {
	pub firm_id: Uuid,
	pub name: Option<String>,
	pub two_gis_firm_id: Option<String>,
	pub category_id: Option<String>,
	pub coords: Option<String>,
	#[serde(rename = "createdTs")]
	pub created_ts: Option<DateTime<Utc>>,
	#[serde(rename = "updatedTs")]
	pub updated_ts: Option<DateTime<Utc>>,
}

// impl<'r> Decode<'r, Postgres> for Firm
// where
//     String: Type<Postgres>,
//     Option<String>: Decode<'r, Postgres>,
//     Option<String>: Type<Postgres>,
// {
//     fn decode(value: PgValueRef<'r>) -> Result<Self, Box<dyn Error + 'static + Send + Sync>> {
//         let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;

//         let firm_id = decoder.try_decode::<Uuid>()?;
//         let category_id = decoder.try_decode::<Uuid>()?;
//         let type_id = decoder.try_decode::<Uuid>()?;
//         let city_id = decoder.try_decode::<Uuid>()?;

//         let two_gis_firm_id = decoder.try_decode::<Option<String>>()?;
//         let name = decoder.try_decode::<Option<String>>()?;
//         let description = decoder.try_decode::<Option<String>>()?;
//         let address = decoder.try_decode::<Option<String>>()?;
//         let floor = decoder.try_decode::<Option<String>>()?;
//         let site = decoder.try_decode::<Option<String>>()?;
//         let default_email = decoder.try_decode::<Option<String>>()?;
//         let default_phone = decoder.try_decode::<Option<String>>()?;
//         let url = decoder.try_decode::<Option<String>>()?;
//         let coords = decoder.try_decode::<Option<String>>()?;

//         let created_ts = decoder.try_decode::<Option<DateTime<Utc>>>()?;
//         let updated_ts = decoder.try_decode::<Option<DateTime<Utc>>>()?;
//         let ts = decoder.try_decode::<Option<TsVector>>()?;

//         Ok(Firm {
// 		        firm_id,
// 		        category_id,
// 		        type_id,
// 		        city_id,
//             two_gis_firm_id,
//             name,
//             description,
//             address,
//             floor,
//             site,
//             default_email,
//             default_phone,
//             url,
//             coords,
//             created_ts,
//             updated_ts,
//             ts,
//         })
//     }
// }

#[allow(non_snake_case)]
#[derive(Debug, sqlx::FromRow, sqlx::Type, Default)]
pub struct Firm {
	pub firm_id: Uuid,
	pub category_id: Uuid,
	pub type_id: Uuid,
	pub city_id: Uuid,
	pub two_gis_firm_id: Option<String>,
	pub name: Option<String>,
	pub description: Option<String>,
	pub address: Option<String>,
	pub floor: Option<String>,
	pub site: Option<String>,
	pub default_email: Option<String>,
	pub default_phone: Option<String>,
	pub url: Option<String>,
	pub rating: Option<String>,
	pub coords: Option<String>,
	pub ts: Option<TsVector>,
	pub created_ts: Option<DateTime<Utc>>,
	pub updated_ts: Option<DateTime<Utc>>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct SaveFirm {
	pub two_gis_firm_id: String,
	pub category_id: Uuid,
	pub type_id: Uuid,
	pub city_id: Uuid,
	pub name: String,
	pub address: String,
	pub coords: String,
	// pub floor: String,
	pub default_phone: String,
	pub site: String,
	// pub default_email: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct FilteredFirm {
	pub firm_id: String,
	pub two_gis_firm_id: Option<String>,
	pub category_id: String,
	pub city_id: String,
	pub name: Option<String>,
	pub description: Option<String>,
	pub address: Option<String>,
	pub site: Option<String>,
	pub rating: Option<String>,
	pub default_phone: Option<String>,
	pub url: Option<String>,
	pub coords: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct UpdateFirmDesc {
	pub firm_id: Uuid,
	pub description: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone, Default)]
pub struct ExtFirmWithOaiDescription {
	pub firm_id: Uuid,
	pub city_id: Uuid,
	pub category_id: Uuid,
	pub name: Option<String>,
	pub address: Option<String>,
	pub site: Option<String>,
	pub default_phone: Option<String>,
	pub oai_description_value: Option<String>,
	pub description: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct ExtFilteredFirmWithOaiDescription {
	pub firm_id: String,
	pub category_id: String,
	pub name: Option<String>,
	pub address: Option<String>,
	pub site: Option<String>,
	pub default_phone: Option<String>,
	pub oai_description_value: Option<String>,
	pub description: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct UpdateFirmAddress {
	pub firm_id: Uuid,
	pub address: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct UpdateFirmRating {
	pub firm_id: Uuid,
	pub rating: String,
}
