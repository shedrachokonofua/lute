use strum::EnumString;
use strum_macros;

#[derive(Debug, Eq, PartialEq, Clone, Hash, strum_macros::Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum JobName {
  ResetCrawlerRequestWindow,
  CrawlNewAlbums,
  ChangeEventSubscriberStatus,
  DeleteExpiredKVItems,
  IndexSpotifyTracks,
  ParserRetry,
  Crawl,
  FetchSpotifyTracksByAlbumIds,
  FetchSpotifyTracksByAlbumSearch,
  GenerateOpenAIEmbeddings,
  GenerateVoyageAIEmbeddings,
  GenerateOllamaEmbeddings,
}
