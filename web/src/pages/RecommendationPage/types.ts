export const RecommendationSettingsFormName = {
  ProfileId: "profileId",
  Count: "recommendationSettings.count",
  IncludePrimaryGenres: "recommendationSettings.includePrimaryGenres",
  ExcludePrimaryGenres: "recommendationSettings.excludePrimaryGenres",
  IncludeSecondaryGenres: "recommendationSettings.includeSecondaryGenres",
  ExcludeSecondaryGenres: "recommendationSettings.excludeSecondaryGenres",
  ExcludeKnownArtists: "recommendationSettings.excludeKnownArtists",
  IncludeLanguages: "recommendationSettings.includeLanguages",
  ExcludeLanguages: "recommendationSettings.excludeLanguages",
  MinReleaseYear: "recommendationSettings.minReleaseYear",
  MaxReleaseYear: "recommendationSettings.maxReleaseYear",
  Method: "method",
  QuantileRankingPrimaryGenresWeight:
    "assessmentSettings.quantileRanking.primaryGenresWeight",
  QuantileRankingSecondaryGenresWeight:
    "assessmentSettings.quantileRanking.secondaryGenresWeight",
  QuantileRankingDescriptorWeight:
    "assessmentSettings.quantileRanking.descriptorWeight",
  QuantileRankingRatingWeight:
    "assessmentSettings.quantileRanking.ratingWeight",
  QuantileRankingRatingCountWeight:
    "assessmentSettings.quantileRanking.ratingCountWeight",
  QuantileRankingDescriptorCountWeight:
    "assessmentSettings.quantileRanking.descriptorCountWeight",
  QuantileRankingCreditTagWeight:
    "assessmentSettings.quantileRanking.creditTagWeight",
  EmbeddingSimilarityEmbeddingKey:
    "assessmentSettings.embeddingSimilarity.embeddingKey",
};

export type RecommendationMethod = "quantile-ranking" | "embedding-similarity";

export interface RecommendationSettingsForm {
  profileId: string | undefined;
  recommendationSettings:
    | {
        count: number | undefined;
        minReleaseYear: number | undefined;
        maxReleaseYear: number | undefined;
        includePrimaryGenres: string[] | undefined;
        excludePrimaryGenres: string[] | undefined;
        includeSecondaryGenres: string[] | undefined;
        excludeSecondaryGenres: string[] | undefined;
        includeLanguages: string[] | undefined;
        excludeLanguages: string[] | undefined;
        excludeKnownArtists: number | undefined;
      }
    | undefined;
  method: RecommendationMethod | undefined;
  assessmentSettings:
    | {
        quantileRanking:
          | {
              primaryGenresWeight: number | undefined;
              secondaryGenresWeight: number | undefined;
              descriptorWeight: number | undefined;
              ratingWeight: number | undefined;
              ratingCountWeight: number | undefined;
              descriptorCountWeight: number | undefined;
              creditTagWeight: number | undefined;
            }
          | undefined;
        embeddingSimilarity:
          | {
              embeddingKey: string | undefined;
            }
          | undefined;
      }
    | undefined;
}
