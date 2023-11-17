import { useRouteLoaderData } from "react-router-dom";
import {
  getAlbumMonitor,
  getAllProfiles,
  getIsSpotifyAuthenticated,
} from "./client";
import { AlbumMonitor, Profile } from "./proto/lute_pb";

export interface AppRemoteContext {
  isSpotifyAuthenticated: boolean;
  profiles: Profile[];
  albumMonitor: AlbumMonitor;
}

export const getRemoteContext = async (): Promise<AppRemoteContext> => {
  const [isSpotifyAuthenticated, profiles, albumMonitor] = await Promise.all([
    getIsSpotifyAuthenticated(),
    getAllProfiles(),
    getAlbumMonitor(),
  ]);
  return {
    isSpotifyAuthenticated,
    profiles,
    albumMonitor,
  };
};

export const useRemoteContext = (): AppRemoteContext =>
  useRouteLoaderData("root") as AppRemoteContext;
