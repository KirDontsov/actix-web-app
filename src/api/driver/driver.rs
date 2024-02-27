use thirtyfour::prelude::*;

pub trait Driver {}

impl dyn Driver {
	pub async fn get_driver() -> Result<WebDriver, WebDriverError> {
		let mut caps = DesiredCapabilities::chrome();
		// без загрузки изображений
		caps.add_chrome_option(
			"prefs",
			serde_json::json!({
				"profile.default_content_settings": {
					"images": 2
				},
				"profile.managed_default_content_settings": {
					"images": 2
				}
			}),
		)?;
		let _ = caps.set_headless();
		let _ = caps.add_chrome_arg("enable-automation");
		let _ = caps.add_chrome_arg("--no-sandbox");
		let _ = caps.add_chrome_arg("--disable-extensions");
		let _ = caps.add_chrome_arg("--dns-prefetch-disable");
		let _ = caps.add_chrome_arg("--disable-gpu");
		let _ = caps.add_chrome_arg("enable-features=NetworkServiceInProcess");
		let driver = WebDriver::new("http://localhost:9515", caps).await;
		driver
	}
}
