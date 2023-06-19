tonic::include_proto!("lute");

pub use crawler_service_server::{CrawlerService, CrawlerServiceServer};
pub use file_service_server::{FileService, FileServiceServer};
pub use lute_server::{Lute, LuteServer};
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("lute_descriptor");
