use crate::AppState;
use actix_web::{get, web, HttpResponse, Responder};
use photon_rs::colour_spaces::darken_hsl;
use photon_rs::conv::box_blur;
use photon_rs::multiple::{blend, watermark};
use photon_rs::native::{open_image, save_image};
use photon_rs::transform::crop;
use photon_rs::PhotonImage;
use tokio::time::{sleep, Duration};

use glob::glob;

#[allow(unreachable_code)]
#[get("/processing/images")]
async fn images_processing_handler(
	data: web::Data<AppState>,
	// _: jwt_auth::JwtMiddleware,
) -> impl Responder {
	println!("start");
	let _: Result<(), Box<dyn std::error::Error>> = match processing(data.clone()).await {
		Ok(x) => Ok(x),
		Err(e) => {
			println!("{:?}", e);
			Err(e)
		}
	};
	// loop {
	// 	let mut needs_to_restart = true;
	// 	if needs_to_restart {
	// 		let _: Result<(), Box<dyn std::error::Error>> = match processing(data.clone()).await {
	// 			Ok(x) => {
	// 				needs_to_restart = false;
	// 				Ok(x)
	// 			}
	// 			Err(e) => {
	// 				println!("{:?}", e);
	// 				let _ = sleep(Duration::from_secs(20)).await;
	// 				needs_to_restart = true;
	// 				Err(e)
	// 			}
	// 		};
	// 	}
	// }
	let json_response = serde_json::json!({
		"status":  "success",
	});
	HttpResponse::Ok().json(json_response)
}

async fn processing(data: web::Data<AppState>) -> Result<(), Box<dyn std::error::Error>> {
	// let counter_id: String = String::from("");
	// let table = String::from("firms");
	// let category_id = uuid::Uuid::parse_str("3ebc7206-6fed-4ea7-a000-27a74e867c9a").unwrap();

	// let firms_count = Count::count_firms_by_category(&data.db, table, category_id)
	// 	.await
	// 	.unwrap_or(0);

	// имея id фирмы как название директории
	// заходим в нее
	// в цикле берем каждое фото
	// обрабатываем
	// и сохраняем обратно с тем же именем

	// let paths = fs::read_dir("output/images").unwrap();

	// for path in paths {
	// 	println!("Name: {}", path.unwrap().path().display());
	// }

	for entry in glob("output/images/**/*.png")? {
		match entry {
			Ok(path) => {
				println!("{}", path.display());
				let mut img = open_image(path.to_str().unwrap()).expect("File should open");
				let width = *&mut img.get_width();
				let height = *&mut img.get_height();

				let mut cropped_img: PhotonImage = crop(
					&mut img,
					width - 121_u32,
					height - 121_u32,
					width,
					height - 51_u32,
				);

				darken_hsl(&mut cropped_img, 0.1_f32);
				box_blur(&mut cropped_img);

				watermark(&mut img, &cropped_img, width - 120_u32, height - 60_u32);
				save_image(img, path.to_str().unwrap()).expect("File should be saved");
			}
			Err(e) => {
				println!("Err: {:?}", e);
			}
		}
	}

	Ok(())
}
