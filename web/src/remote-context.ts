import { useRouteLoaderData } from "react-router-dom";
import {
  getAggregatedGenres,
  getAggregatedLanguages,
  getAllProfiles,
  getIsSpotifyAuthenticated,
} from "./client";
import { GenreAggregate, LanguageAggregate, Profile } from "./proto/lute_pb";

export interface AppRemoteContext {
  isSpotifyAuthenticated: boolean;
  profiles: Profile[];
  aggregatedGenres: GenreAggregate[];
  aggregatedLanguages: LanguageAggregate[];
}

export const getRemoteContext = async (): Promise<AppRemoteContext> => {
  const [
    isSpotifyAuthenticated,
    profiles,
    aggregatedGenres,
    aggregatedLanguages,
  ] = await Promise.all([
    getIsSpotifyAuthenticated(),
    getAllProfiles(),
    getAggregatedGenres(),
    getAggregatedLanguages(),
  ]);
  return {
    isSpotifyAuthenticated,
    profiles,
    aggregatedGenres,
    aggregatedLanguages,
  };
};

export const useRemoteContext = (): AppRemoteContext =>
  useRouteLoaderData("root") as AppRemoteContext;
