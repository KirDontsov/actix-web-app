pub mod description_processing;
pub mod images_processing;
pub mod reviews_processing;

pub use self::description_processing::description_processing_handler;
pub use self::images_processing::images_processing_handler;
pub use self::reviews_processing::reviews_processing_handler;
