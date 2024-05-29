import { ChannelCredentials } from "@grpc/grpc-js";
import { config } from "./config";
import { lute as core } from "./proto/lute";
import { google } from "./proto/google/protobuf/empty";

export const lute = {
  albums: new core.AlbumServiceClient(
    config.coreUrl,
    ChannelCredentials.createInsecure()
  ),
  recommendations: new core.RecommendationServiceClient(
    config.coreUrl,
    ChannelCredentials.createInsecure()
  ),
};

export const getAlbumMonitor = async () => {
  return await lute.albums.GetMonitor(new google.protobuf.Empty());
};

export interface RecommendationFilters {
  includePrimaryGenres: string[];
  excludePrimaryGenres: string[];
  includeSecondaryGenres: string[];
  excludeSecondaryGenres: string[];
  includeLanguages: string[];
  excludeLanguages: string[];
  includeDescriptors: string[];
  excludeDescriptors: string[];
  minReleaseYear: number;
  maxReleaseYear: number;
}

export interface RecommendationSettings {
  profileId: string;
  filters: RecommendationFilters;
  limit: number;
}

export const recommendAlbums = async (settings: RecommendationSettings) => {
  const request = new core.RecommendAlbumsRequest({
    profileId: settings.profileId,
    recommendationSettings: new core.AlbumRecommendationSettings({
      count: settings.limit,
      includePrimaryGenres: settings.filters.includePrimaryGenres,
      excludePrimaryGenres: settings.filters.excludePrimaryGenres,
      includeSecondaryGenres: settings.filters.includeSecondaryGenres,
      excludeSecondaryGenres: settings.filters.excludeSecondaryGenres,
      includeLanguages: settings.filters.includeLanguages,
      excludeLanguages: settings.filters.excludeLanguages,
      minReleaseYear: settings.filters.minReleaseYear,
      maxReleaseYear: settings.filters.maxReleaseYear,
    }),
    assessmentSettings: new core.AlbumAssessmentSettings({
      embeddingSimilaritySettings:
        new core.EmbeddingSimilarityAlbumAssessmentSettings({
          embeddingKey: "voyageai-default",
        }),
    }),
  });
  const recommendations = await lute.recommendations.RecommendAlbums(request);
  return recommendations.recommendations.map(
    (recommendation) => recommendation.toObject().album
  );
};
