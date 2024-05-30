import { ChannelCredentials } from "@grpc/grpc-js";
import { config } from "./config";
import { lute as core } from "./proto/lute";
import { google } from "./proto/google/protobuf/empty";

export const lute = {
  albums: new core.AlbumServiceClient(
    config.coreUrl,
    ChannelCredentials.createInsecure(),
    {
      "grpc.max_receive_message_length": 1024 * 1024 * 100,
    }
  ),
  recommendations: new core.RecommendationServiceClient(
    config.coreUrl,
    ChannelCredentials.createInsecure()
  ),
};

export const getAlbumMonitor = async () => {
  return (await lute.albums.GetMonitor(new google.protobuf.Empty())).monitor;
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
      includeDescriptors: settings.filters.includeDescriptors,
      excludeDescriptors: settings.filters.excludeDescriptors,
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

export interface Pagination {
  offset?: number;
  limit?: number;
}

const toPagination = (pagination?: Pagination) => {
  return new core.SearchPagination({
    offset: pagination?.offset,
    limit: pagination?.limit,
  });
};
export interface AlbumSearchParams {
  query: AlbumQuery;
  pagination: Pagination;
}

export interface AlbumQuery {
  text: string;
  excludeFileNames: string[];
  includePrimaryGenres: string[];
  excludePrimaryGenres: string[];
  includeSecondaryGenres: string[];
  excludeSecondaryGenres: string[];
  includeLanguages: string[];
  excludeLanguages: string[];
  includeDescriptors: string[];
  excludeDescriptors: string[];
  minReleaseYear?: number;
  maxReleaseYear?: number;
}

const toAlbumSearchQuery = (query?: AlbumQuery) => {
  return new core.AlbumSearchQuery({
    text: query?.text,
    excludeFileNames: query?.excludeFileNames,
    includePrimaryGenres: query?.includePrimaryGenres,
    excludePrimaryGenres: query?.excludePrimaryGenres,
    includeSecondaryGenres: query?.includeSecondaryGenres,
    excludeSecondaryGenres: query?.excludeSecondaryGenres,
    includeLanguages: query?.includeLanguages,
    excludeLanguages: query?.excludeLanguages,
    includeDescriptors: query?.includeDescriptors,
    excludeDescriptors: query?.excludeDescriptors,
    minReleaseYear: query?.minReleaseYear,
    maxReleaseYear: query?.maxReleaseYear,
  });
};

export const searchAlbums = async (params: AlbumSearchParams) => {
  const request = new core.SearchAlbumsRequest({
    query: toAlbumSearchQuery(params.query),
    pagination: toPagination(params.pagination),
  });
  const results = await lute.albums.SearchAlbums(request);
  return results.albums.map((album) => album.toObject());
};

export interface FindSimilarAlbumsParams {
  fileName: string;
  filters: AlbumQuery;
  limit: number;
}

export const findSimilarAlbums = async (params: FindSimilarAlbumsParams) => {
  const request = new core.FindSimilarAlbumsRequest({
    fileName: params.fileName,
    filters: toAlbumSearchQuery(params.filters),
    embeddingKey: "voyageai-default",
    limit: params.limit,
  });
  const results = await lute.albums.FindSimilarAlbums(request);
  return results.albums.map((album) => album.toObject());
};
