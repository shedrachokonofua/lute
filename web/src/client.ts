import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { RecommendationSettingsForm } from "./pages/recommendations/types";
import { SimilarAlbumsForm } from "./pages/similar-albums/types";
import {
  AlbumServiceClient,
  ProfileServiceClient,
  RecommendationServiceClient,
  SpotifyServiceClient,
} from "./proto/LuteServiceClientPb";
import {
  Album,
  AlbumAssessmentSettings,
  AlbumMonitor,
  AlbumRecommendation,
  AlbumRecommendationSettings,
  AlbumSearchQuery,
  ClearPendingSpotifyImportsRequest,
  CreateProfileRequest,
  DeleteProfileRequest,
  EmbeddingSimilarityAlbumAssessmentSettings,
  FindSimilarAlbumsRequest,
  GetAlbumRequest,
  GetManyAlbumsRequest,
  GetPendingSpotifyImportsReply,
  GetPendingSpotifyImportsRequest,
  GetProfileRequest,
  GetProfileSummaryRequest,
  HandleAuthorizationCodeRequest,
  ImportSavedSpotifyTracksRequest,
  ImportSpotifyPlaylistTracksRequest,
  Profile,
  ProfileSummary,
  PutAlbumOnProfileRequest,
  QuantileRankAlbumAssessmentSettings,
  RecommendAlbumsRequest,
  RemoveAlbumFromProfileRequest,
  SearchAlbumsReply,
  SearchAlbumsRequest,
  SearchPagination,
} from "./proto/lute_pb";

const coreUrl = "http://localhost:22000";
const client = {
  spotify: new SpotifyServiceClient(coreUrl),
  profile: new ProfileServiceClient(coreUrl),
  album: new AlbumServiceClient(coreUrl),
  recommendation: new RecommendationServiceClient(coreUrl),
};

export const getIsSpotifyAuthenticated = async (): Promise<boolean> => {
  const response = await client.spotify.isAuthorized(new Empty(), null);
  return response.getAuthorized();
};

export const getSpotifyAuthUrl = async (): Promise<string> => {
  const response = await client.spotify.getAuthorizationUrl(new Empty(), null);
  return response.getUrl();
};

export const handleSpotifyAuthCode = async (code: string): Promise<void> => {
  const request = new HandleAuthorizationCodeRequest();
  request.setCode(code);
  await client.spotify.handleAuthorizationCode(request, null);
};

export const getAllProfiles = async (): Promise<Profile[]> => {
  const response = await client.profile.getAllProfiles(new Empty(), null);
  return response.getProfilesList();
};

export const getEmbeddingKeys = async (): Promise<string[]> => {
  const response = await client.album.getEmbeddingKeys(new Empty(), null);
  return response.getKeysList();
};

export const settingsToRecommendationRequest = (
  settings: RecommendationSettingsForm,
): RecommendAlbumsRequest | null => {
  if (!settings.profileId) {
    return null;
  }
  const request = new RecommendAlbumsRequest();
  request.setProfileId(settings.profileId);
  const recommedationSettings = new AlbumRecommendationSettings();
  if (settings.recommendationSettings?.count) {
    recommedationSettings.setCount(settings.recommendationSettings.count);
  }
  if (settings.recommendationSettings?.minReleaseYear) {
    recommedationSettings.setMinReleaseYear(
      settings.recommendationSettings.minReleaseYear,
    );
  }
  if (settings.recommendationSettings?.maxReleaseYear) {
    recommedationSettings.setMaxReleaseYear(
      settings.recommendationSettings.maxReleaseYear,
    );
  }
  if (settings.recommendationSettings?.includePrimaryGenres) {
    recommedationSettings.setIncludePrimaryGenresList(
      settings.recommendationSettings.includePrimaryGenres,
    );
  }
  if (settings.recommendationSettings?.includeSecondaryGenres) {
    recommedationSettings.setIncludeSecondaryGenresList(
      settings.recommendationSettings.includeSecondaryGenres,
    );
  }
  if (settings.recommendationSettings?.excludePrimaryGenres) {
    recommedationSettings.setExcludePrimaryGenresList(
      settings.recommendationSettings.excludePrimaryGenres,
    );
  }
  if (settings.recommendationSettings?.excludeSecondaryGenres) {
    recommedationSettings.setExcludeSecondaryGenresList(
      settings.recommendationSettings.excludeSecondaryGenres,
    );
  }
  if (settings.recommendationSettings?.includeLanguages) {
    recommedationSettings.setIncludeLanguagesList(
      settings.recommendationSettings.includeLanguages,
    );
  }
  if (settings.recommendationSettings?.excludeLanguages) {
    recommedationSettings.setExcludeLanguagesList(
      settings.recommendationSettings.excludeLanguages,
    );
  }
  if (settings.recommendationSettings?.excludeKnownArtists) {
    recommedationSettings.setExcludeKnownArtists(true);
  }
  request.setRecommendationSettings(recommedationSettings);

  const assessmentSettings = new AlbumAssessmentSettings();
  if (settings.assessmentSettings?.quantileRanking) {
    const quantileRankSettings = new QuantileRankAlbumAssessmentSettings();
    const {
      primaryGenresWeight,
      secondaryGenresWeight,
      descriptorWeight,
      ratingCountWeight,
      ratingWeight,
      descriptorCountWeight,
      creditTagWeight,
    } = settings.assessmentSettings.quantileRanking;
    if (primaryGenresWeight !== undefined) {
      quantileRankSettings.setPrimaryGenreWeight(primaryGenresWeight);
    }
    if (secondaryGenresWeight !== undefined) {
      quantileRankSettings.setSecondaryGenreWeight(secondaryGenresWeight);
    }
    if (descriptorWeight !== undefined) {
      quantileRankSettings.setDescriptorWeight(descriptorWeight);
    }
    if (ratingCountWeight !== undefined) {
      quantileRankSettings.setRatingCountWeight(ratingCountWeight);
    }
    if (ratingWeight !== undefined) {
      quantileRankSettings.setRatingWeight(ratingWeight);
    }
    if (descriptorCountWeight !== undefined) {
      quantileRankSettings.setDescriptorCountWeight(descriptorCountWeight);
    }
    if (creditTagWeight !== undefined) {
      quantileRankSettings.setCreditTagWeight(creditTagWeight);
    }
    assessmentSettings.setQuantileRankSettings(quantileRankSettings);
  }
  if (settings.assessmentSettings?.embeddingSimilarity) {
    const embeddingSimilaritySettings =
      new EmbeddingSimilarityAlbumAssessmentSettings();
    const { embeddingKey } = settings.assessmentSettings.embeddingSimilarity;
    if (embeddingKey !== undefined) {
      embeddingSimilaritySettings.setEmbeddingKey(embeddingKey);
    }
    assessmentSettings.setEmbeddingSimilaritySettings(
      embeddingSimilaritySettings,
    );
  }
  request.setAssessmentSettings(assessmentSettings);

  return request;
};

export const getAlbumRecommendations = async (
  settings: RecommendationSettingsForm,
): Promise<AlbumRecommendation[]> => {
  const request = settingsToRecommendationRequest(settings);
  if (!request) {
    throw new Error("Invalid settings");
  }
  const response = await client.recommendation.recommendAlbums(request, null);
  return response.getRecommendationsList();
};

export const getDefaultQuantileRankAlbumAssessmentSettings =
  async (): Promise<QuantileRankAlbumAssessmentSettings> => {
    const response =
      await client.recommendation.defaultQuantileRankAlbumAssessmentSettings(
        new Empty(),
        null,
      );
    const settings = response.getSettings();
    if (!settings) {
      throw new Error("No settings returned");
    }
    return settings;
  };

export const getProfile = async (id: string): Promise<Profile | undefined> => {
  const request = new GetProfileRequest();
  request.setId(id);
  const response = await client.profile.getProfile(request, null);
  return response.getProfile()!;
};

export const getProfileSummary = async (
  id: string,
): Promise<ProfileSummary | undefined> => {
  const request = new GetProfileSummaryRequest();
  request.setId(id);
  const response = await client.profile.getProfileSummary(request, null);
  return response.getSummary();
};

export const createProfile = async (
  id: string,
  name: string,
): Promise<Profile> => {
  const request = new CreateProfileRequest();
  request.setId(id);
  request.setName(name);
  const response = await client.profile.createProfile(request, null);
  return response.getProfile()!;
};

export const deleteProfile = async (id: string): Promise<void> => {
  const request = new DeleteProfileRequest();
  request.setId(id);
  await client.profile.deleteProfile(request, null);
};

export const getAlbum = async (fileName: string): Promise<Album> => {
  const request = new GetAlbumRequest();
  request.setFileName(fileName);
  const response = await client.album.getAlbum(request, null);
  return response.getAlbum()!;
};

export const getManyAlbums = async (fileNames: string[]): Promise<Album[]> => {
  const request = new GetManyAlbumsRequest();
  request.setFileNamesList(fileNames);
  const response = await client.album.getManyAlbums(request, null);
  return response.getAlbumsList();
};

export interface AlbumSearchQueryParams {
  text?: string;
  exactName?: string;
  includeFileNames?: string[];
  excludeFileNames?: string[];
  includeArtists?: string[];
  excludeArtists?: string[];
  includePrimaryGenres?: string[];
  excludePrimaryGenres?: string[];
  includeSecondaryGenres?: string[];
  excludeSecondaryGenres?: string[];
  includeLanguages?: string[];
  excludeLanguages?: string[];
  includeDescriptors?: string[];
  minPrimaryGenreCount?: number;
  minSecondaryGenreCount?: number;
  minDescriptorCount?: number;
  minReleaseYear?: number;
  maxReleaseYear?: number;
  includeDuplicates?: boolean;
}

export interface AlbumSearchPaginationParams {
  offset?: number;
  limit?: number;
}

export const searchAlbums = async (
  queryParams: AlbumSearchQueryParams,
  paginationParams?: AlbumSearchPaginationParams,
): Promise<SearchAlbumsReply> => {
  const query = new AlbumSearchQuery();
  if (queryParams.text) {
    query.setText(queryParams.text);
  }
  if (queryParams.exactName) {
    query.setExactName(queryParams.exactName);
  }
  if (queryParams.includeFileNames) {
    query.setIncludeFileNamesList(queryParams.includeFileNames);
  }
  if (queryParams.excludeFileNames) {
    query.setExcludeFileNamesList(queryParams.excludeFileNames);
  }
  if (queryParams.includeArtists) {
    query.setIncludeArtistsList(queryParams.includeArtists);
  }
  if (queryParams.excludeArtists) {
    query.setExcludeArtistsList(queryParams.excludeArtists);
  }
  if (queryParams.includePrimaryGenres) {
    query.setIncludePrimaryGenresList(queryParams.includePrimaryGenres);
  }
  if (queryParams.excludePrimaryGenres) {
    query.setExcludePrimaryGenresList(queryParams.excludePrimaryGenres);
  }
  if (queryParams.includeSecondaryGenres) {
    query.setIncludeSecondaryGenresList(queryParams.includeSecondaryGenres);
  }
  if (queryParams.excludeSecondaryGenres) {
    query.setExcludeSecondaryGenresList(queryParams.excludeSecondaryGenres);
  }
  if (queryParams.includeLanguages) {
    query.setIncludeLanguagesList(queryParams.includeLanguages);
  }
  if (queryParams.excludeLanguages) {
    query.setExcludeLanguagesList(queryParams.excludeLanguages);
  }
  if (queryParams.includeDescriptors) {
    query.setIncludeDescriptorsList(queryParams.includeDescriptors);
  }
  if (queryParams.minPrimaryGenreCount) {
    query.setMinPrimaryGenreCount(queryParams.minPrimaryGenreCount);
  }
  if (queryParams.minSecondaryGenreCount) {
    query.setMinSecondaryGenreCount(queryParams.minSecondaryGenreCount);
  }
  if (queryParams.minDescriptorCount) {
    query.setMinDescriptorCount(queryParams.minDescriptorCount);
  }
  if (queryParams.minReleaseYear) {
    query.setMinReleaseYear(queryParams.minReleaseYear);
  }
  if (queryParams.maxReleaseYear) {
    query.setMaxReleaseYear(queryParams.maxReleaseYear);
  }
  if (queryParams.includeDuplicates) {
    query.setIncludeDuplicates(queryParams.includeDuplicates);
  }

  const pagination = new SearchPagination();
  if (paginationParams?.offset) {
    pagination.setOffset(paginationParams.offset);
  }
  if (paginationParams?.limit) {
    pagination.setLimit(paginationParams.limit);
  }

  const request = new SearchAlbumsRequest();
  request.setQuery(query);
  request.setPagination(pagination);
  return await client.album.searchAlbums(request, null);
};

export const putAlbumOnProfile = async (
  profileId: string,
  fileName: string,
  factor: number,
): Promise<Profile> => {
  const request = new PutAlbumOnProfileRequest();
  request.setProfileId(profileId);
  request.setFileName(fileName);
  request.setFactor(factor);
  const response = await client.profile.putAlbumOnProfile(request, null);
  return response.getProfile()!;
};

export const removeAlbumFromProfile = async (
  profileId: string,
  fileName: string,
): Promise<void> => {
  const request = new RemoveAlbumFromProfileRequest();
  request.setProfileId(profileId);
  request.setFileName(fileName);
  await client.profile.removeAlbumFromProfile(request, null);
};

export const getPendingSpotifyImports = async (
  profileId: string,
): Promise<GetPendingSpotifyImportsReply> => {
  const request = new GetPendingSpotifyImportsRequest();
  request.setProfileId(profileId);
  const response = await client.profile.getPendingSpotifyImports(request, null);
  return response;
};

export const importSavedSpotifyTracks = async (
  profileId: string,
): Promise<void> => {
  const request = new ImportSavedSpotifyTracksRequest();
  request.setProfileId(profileId);
  await client.profile.importSavedSpotifyTracks(request, null);
};

export const importSpotifyPlaylistTracks = async (
  profileId: string,
  playlistId: string,
): Promise<void> => {
  const request = new ImportSpotifyPlaylistTracksRequest();
  request.setProfileId(profileId);
  request.setPlaylistId(playlistId);
  await client.profile.importSpotifyPlaylistTracks(request, null);
};

export const clearPendingSpotifyImports = async (
  profileId: string,
): Promise<void> => {
  const request = new ClearPendingSpotifyImportsRequest();
  request.setProfileId(profileId);
  await client.profile.clearPendingSpotifyImports(request, null);
};

export const getAlbumMonitor = async (): Promise<AlbumMonitor> => {
  const response = await client.album.getMonitor(new Empty(), null);
  return response.getMonitor()!;
};

export const findSimilarAlbums = async ({
  fileName,
  embeddingKey,
  filters,
  limit,
}: SimilarAlbumsForm): Promise<Album[]> => {
  const request = new FindSimilarAlbumsRequest();
  if (!fileName || !embeddingKey) {
    throw new Error("Invalid settings");
  }
  request.setFileName(fileName);
  request.setEmbeddingKey(embeddingKey);

  if (limit) {
    request.setLimit(limit);
  }

  if (filters) {
  }

  const response = await client.album.findSimilarAlbums(request, null);
  return response.getAlbumsList();
};
