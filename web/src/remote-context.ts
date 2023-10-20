import { useRouteLoaderData } from "react-router-dom";
import {
  getAggregatedGenres,
  getAggregatedLanguages,
  getAllProfiles,
} from "./client";
import { GenreAggregate, LanguageAggregate, Profile } from "./proto/lute_pb";

export interface AppRemoteContext {
  profiles: Profile[];
  aggregatedGenres: GenreAggregate[];
  aggregatedLanguages: LanguageAggregate[];
}

export const getRemoteContext = async (): Promise<AppRemoteContext> => {
  const [profiles, aggregatedGenres, aggregatedLanguages] = await Promise.all([
    getAllProfiles(),
    getAggregatedGenres(),
    getAggregatedLanguages(),
  ]);
  return {
    profiles,
    aggregatedGenres,
    aggregatedLanguages,
  };
};

export const useRemoteContext = (): AppRemoteContext =>
  useRouteLoaderData("root") as AppRemoteContext;
