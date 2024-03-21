import { AlbumSearchFiltersForm } from "../../forms";

export type RecommendationMethod = "quantile-ranking" | "embedding-similarity";

export interface RecommendationSettingsForm {
  profileId?: string;
  recommendationSettings?: AlbumSearchFiltersForm;
  method?: RecommendationMethod;
  assessmentSettings?: {
    quantileRanking?: {
      primaryGenresWeight?: number;
      secondaryGenresWeight?: number;
      descriptorWeight?: number;
      ratingWeight?: number;
      ratingCountWeight?: number;
      descriptorCountWeight?: number;
      creditTagWeight?: number;
    };
    embeddingSimilarity?: {
      embeddingKey?: string;
    };
  };
}
