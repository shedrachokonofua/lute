export interface AlbumSearchFiltersForm {
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

export const coerceToUndefined = <
  T extends string | Record<string, unknown> | null | undefined,
>(
  value: T,
): NonNullable<T> | undefined => {
  if (value === null || value === undefined) {
    return undefined;
  }
  if (typeof value === "string") {
    return value.trim() === "" ? undefined : value;
  }
  if (typeof value === "object") {
    return Object.keys(value).length === 0 ||
      Object.values(value).every((v) => !v)
      ? undefined
      : value;
  }
  return value;
};

export const toNumber = (value: string | null | undefined) => {
  return value ? Number(value) : undefined;
};

export const parseAlbumSearchFiltersForm = (
  url: URL,
): AlbumSearchFiltersForm | undefined =>
  coerceToUndefined({
    count: toNumber(coerceToUndefined(url.searchParams.get(FormName.Count))),
    minReleaseYear: toNumber(
      coerceToUndefined(url.searchParams.get(FormName.MinReleaseYear)),
    ),
    maxReleaseYear: toNumber(
      coerceToUndefined(url.searchParams.get(FormName.MaxReleaseYear)),
    ),
    includePrimaryGenres: coerceToUndefined(
      url.searchParams.get(FormName.IncludePrimaryGenres),
    )?.split(","),
    excludePrimaryGenres: coerceToUndefined(
      url.searchParams.get(FormName.ExcludePrimaryGenres),
    )?.split(","),
    includeSecondaryGenres: coerceToUndefined(
      url.searchParams.get(FormName.IncludeSecondaryGenres),
    )?.split(","),
    excludeSecondaryGenres: coerceToUndefined(
      url.searchParams.get(FormName.ExcludeSecondaryGenres),
    )?.split(","),
    includeLanguages: coerceToUndefined(
      url.searchParams.get(FormName.IncludeLanguages),
    )?.split(","),
    excludeLanguages: coerceToUndefined(
      url.searchParams.get(FormName.ExcludeLanguages),
    )?.split(","),
    excludeKnownArtists: toNumber(
      coerceToUndefined(url.searchParams.get(FormName.ExcludeKnownArtists)),
    ),
  });

export const FormName = {
  FileName: "fileName",
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
