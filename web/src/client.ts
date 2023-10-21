import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { RecommendationSettingsForm } from "./pages/RecommendationPage/types";
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
  EmbeddingSimilarityAlbumAssessmentSettings,
  GenreAggregate,
  HandleAuthorizationCodeRequest,
  LanguageAggregate,
  Profile,
  QuantileRankAlbumAssessmentSettings,
  RecommendAlbumsRequest,
} from "./proto/lute_pb";

const coreUrl = "http://0.0.0.0:22000";
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

export const getAggregatedGenres = async (): Promise<GenreAggregate[]> => {
  const response = await client.album.getAggregatedGenres(new Empty(), null);
  return response.getGenresList();
};

export const getAggregatedLanguages = async (): Promise<
  LanguageAggregate[]
> => {
  const response = await client.album.getAggregatedLanguages(new Empty(), null);
  return response.getLanguagesList();
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
