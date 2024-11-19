import { ChannelCredentials } from "@grpc/grpc-js";
import { lute } from "../proto/lute";
import { google } from "../proto/google/protobuf/empty";

const coreUrl = "pc:22000";

export const client = {
  albums: new lute.AlbumServiceClient(
    coreUrl,
    ChannelCredentials.createInsecure(),
    {
      "grpc.max_receive_message_length": 1024 * 1024 * 100,
    }
  ),
  artists: new lute.ArtistServiceClient(
    coreUrl,
    ChannelCredentials.createInsecure(),
    {
      "grpc.max_receive_message_length": 1024 * 1024 * 100,
    }
  ),
  crawler: new lute.CrawlerServiceClient(
    coreUrl,
    ChannelCredentials.createInsecure()
  ),
  parser: new lute.ParserServiceClient(
    coreUrl,
    ChannelCredentials.createInsecure()
  ),
};

export const getAlbumMonitor = async () => {
  return (await client.albums.GetMonitor(new google.protobuf.Empty())).monitor;
};

export const enqueueCrawl = async (
  fileName: string,
  priority: lute.Priority
) => {
  return client.crawler.Enqueue(
    new lute.EnqueueRequest({ fileName, priority })
  );
};

export const parseFileOnContentStore = async (fileName: string) => {
  return client.parser.ParseFileOnContentStore(
    new lute.ParseFileOnContentStoreRequest({ fileName })
  );
};

type Pagination = {
  limit: number;
  offset: number;
};

export const getArtists = async (pagination: Pagination) => {
  return await client.artists.SearchArtists(
    new lute.SearchArtistsRequest({
      pagination: lute.SearchPagination.fromObject(pagination),
    })
  );
};

export const getAllArtists = async () => {
  const artists = [];
  let offset = 0;
  let limit = 10000;
  let response;

  while (true) {
    try {
      response = await getArtists({ offset, limit });
    } catch (e) {
      console.error(e);
      break;
    }
    artists.push(...response.artists);
    limit = Math.min(limit, response.total - offset);
    offset += limit;
    if (artists.length >= response.total || response.artists.length === 0) {
      break;
    }
    console.log(`Fetched ${artists.length} artists`);
  }

  return artists;
};
