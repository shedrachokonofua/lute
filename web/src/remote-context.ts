import { useRouteLoaderData } from "react-router-dom";
import {
  getAlbumMonitor,
  getAllProfiles,
  getEmbeddingKeys,
  getIsSpotifyAuthenticated,
} from "./client";
import { AlbumMonitor, Profile } from "./proto/lute_pb";

export interface AppRemoteContext {
  isSpotifyAuthenticated: boolean;
  profiles: Profile[];
  albumMonitor: AlbumMonitor;
  embeddingKeys: string[];
}

export const getRemoteContext = async (): Promise<AppRemoteContext> => {
  const [isSpotifyAuthenticated, profiles, albumMonitor, embeddingKeys] =
    await Promise.all([
      getIsSpotifyAuthenticated(),
      getAllProfiles(),
      getAlbumMonitor(),
      getEmbeddingKeys(),
    ]);

  return {
    isSpotifyAuthenticated,
    profiles,
    albumMonitor,
    embeddingKeys,
  };
};

export const useRemoteContext = (): AppRemoteContext =>
  useRouteLoaderData("root") as AppRemoteContext;
