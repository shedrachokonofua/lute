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
