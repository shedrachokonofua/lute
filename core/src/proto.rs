tonic::include_proto!("lute");

pub use album_service_server::{AlbumService, AlbumServiceServer};
pub use crawler_service_server::{CrawlerService, CrawlerServiceServer};
pub use event_service_server::{EventService, EventServiceServer};
pub use file_service_server::{FileService, FileServiceServer};
pub use lookup_service_server::{LookupService, LookupServiceServer};
pub use lute_server::{Lute, LuteServer};
pub use operations_service_server::{OperationsService, OperationsServiceServer};
pub use parser_service_server::{ParserService, ParserServiceServer};
pub use profile_service_server::{ProfileService, ProfileServiceServer};
pub use recommendation_service_server::{RecommendationService, RecommendationServiceServer};
pub use scheduler_service_server::{SchedulerService, SchedulerServiceServer};
pub use spotify_service_server::{SpotifyService, SpotifyServiceServer};
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("lute_descriptor");
