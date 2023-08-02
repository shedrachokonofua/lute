import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { RecommendationSettingsForm } from "./pages/RecommendationPage/RecommendationSettings";
import {
  AlbumServiceClient,
  ProfileServiceClient,
  RecommendationServiceClient,
  SpotifyServiceClient,
} from "./proto/LuteServiceClientPb";
import {
  AlbumAssessmentSettings,
  AlbumRecommendation,
  AlbumRecommendationSettings,
  GenreAggregate,
  HandleAuthorizationCodeRequest,
  Profile,
  QuantileRankAlbumAssessmentSettings,
  RecommendAlbumsRequest,
} from "./proto/lute_pb";

const coreUrl = "http://localhost:22000";
const client = {
  spotify: new SpotifyServiceClient(coreUrl),
  profile: new ProfileServiceClient(coreUrl),
  album: new AlbumServiceClient(coreUrl),
  recommendation: new RecommendationServiceClient(coreUrl),
};

export const isSpotifyAuthorized = async (): Promise<boolean> => {
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

export const getAggregatedGenres = async (): Promise<GenreAggregate[]> => {
  const response = await client.album.getAggregatedGenres(new Empty(), null);
  return response.getGenresList();
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
    } = settings.assessmentSettings.quantileRanking;
    if (primaryGenresWeight) {
      quantileRankSettings.setPrimaryGenreWeight(primaryGenresWeight);
    }
    if (secondaryGenresWeight) {
      quantileRankSettings.setSecondaryGenreWeight(secondaryGenresWeight);
    }
    if (descriptorWeight) {
      quantileRankSettings.setDescriptorWeight(descriptorWeight);
    }
    if (ratingCountWeight) {
      quantileRankSettings.setRatingCountWeight(ratingCountWeight);
    }
    if (ratingWeight) {
      quantileRankSettings.setRatingWeight(ratingWeight);
    }
    assessmentSettings.setQuantileRankSettings(quantileRankSettings);
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
