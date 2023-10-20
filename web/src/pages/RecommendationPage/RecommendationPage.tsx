import { Grid, Stack } from "@mantine/core";
import React from "react";
import {
  Await,
  LoaderFunctionArgs,
  defer,
  useLoaderData,
  useRouteError,
} from "react-router-dom";
import {
  getAggregatedGenres,
  getAggregatedLanguages,
  getAlbumRecommendations,
  getAllProfiles,
  getDefaultQuantileRankAlbumAssessmentSettings,
  getEmbeddingKeys,
} from "../../client";
import {
  AlbumRecommendation,
  GenreAggregate,
  LanguageAggregate,
  Profile,
  QuantileRankAlbumAssessmentSettings,
} from "../../proto/lute_pb";
import { AlbumRecommendationItem } from "./AlbumRecommendationItem";
import { RecommendationSettings } from "./RecommendationSettings";
import {
  RecommendationMethod,
  RecommendationSettingsForm,
  RecommendationSettingsFormName,
} from "./types";

function ErrorBoundary() {
  let error = useRouteError();
  console.error(error);
  return <div>Dang! Something went wrong.</div>;
}

const coerceToUndefined = <
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

const toNumber = (value: string | null | undefined) => {
  return value ? Number(value) : undefined;
};

interface RecommendationSettingsLoaderData {
  profiles: Profile[];
  aggregatedGenres: GenreAggregate[];
  aggregatedLanguages: LanguageAggregate[];
  embeddingKeys: string[];
  settings: RecommendationSettingsForm | null;
  recommendations: AlbumRecommendation[] | null;
  defaultQuantileRankAlbumAssessmentSettings: QuantileRankAlbumAssessmentSettings;
}

export const recommendationPageLoader = async ({
  request,
}: LoaderFunctionArgs) => {
  const url = new URL(request.url);
  const profileId = url.searchParams.get(
    RecommendationSettingsFormName.ProfileId,
  );
  const assessmentMethod = url.searchParams.get(
    RecommendationSettingsFormName.Method,
  );
  const defaultQuantileRankAlbumAssessmentSettings =
    await getDefaultQuantileRankAlbumAssessmentSettings();
  const assessmentSettings =
    assessmentMethod === "quantile-ranking"
      ? coerceToUndefined({
          quantileRanking: {
            primaryGenresWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingPrimaryGenresWeight,
                ),
              ),
            ),
            secondaryGenresWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingSecondaryGenresWeight,
                ),
              ),
            ),
            descriptorWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingDescriptorWeight,
                ),
              ),
            ),
            ratingWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingRatingWeight,
                ),
              ),
            ),
            ratingCountWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingRatingCountWeight,
                ),
              ),
            ),
            descriptorCountWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingDescriptorCountWeight,
                ),
              ),
            ),
            creditTagWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingCreditTagWeight,
                ),
              ),
            ),
          },
          embeddingSimilarity: undefined,
        })
      : assessmentMethod === "embedding-similarity"
      ? coerceToUndefined({
          quantileRanking: undefined,
          embeddingSimilarity: {
            embeddingKey: coerceToUndefined(
              url.searchParams.get(
                RecommendationSettingsFormName.EmbeddingSimilarityEmbeddingKey,
              ),
            ),
          },
        })
      : undefined;

  const settings = profileId
    ? {
        profileId,
        method: coerceToUndefined(assessmentMethod) as
          | RecommendationMethod
          | undefined,
        recommendationSettings: coerceToUndefined({
          count: toNumber(
            coerceToUndefined(
              url.searchParams.get(RecommendationSettingsFormName.Count),
            ),
          ),
          minReleaseYear: toNumber(
            coerceToUndefined(
              url.searchParams.get(
                RecommendationSettingsFormName.MinReleaseYear,
              ),
            ),
          ),
          maxReleaseYear: toNumber(
            coerceToUndefined(
              url.searchParams.get(
                RecommendationSettingsFormName.MaxReleaseYear,
              ),
            ),
          ),
          includePrimaryGenres: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.IncludePrimaryGenres,
            ),
          )?.split(","),
          excludePrimaryGenres: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.ExcludePrimaryGenres,
            ),
          )?.split(","),
          includeSecondaryGenres: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.IncludeSecondaryGenres,
            ),
          )?.split(","),
          excludeSecondaryGenres: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.ExcludeSecondaryGenres,
            ),
          )?.split(","),
          includeLanguages: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.IncludeLanguages,
            ),
          )?.split(","),
          excludeLanguages: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.ExcludeLanguages,
            ),
          )?.split(","),
          excludeKnownArtists: toNumber(
            coerceToUndefined(
              url.searchParams.get(
                RecommendationSettingsFormName.ExcludeKnownArtists,
              ),
            ),
          ),
        }),
        assessmentSettings,
      }
    : null;

  const recommendations = settings ? getAlbumRecommendations(settings) : null;

  const [profiles, aggregatedGenres, aggregatedLanguages, embeddingKeys] =
    await Promise.all([
      getAllProfiles(),
      getAggregatedGenres(),
      getAggregatedLanguages(),
      getEmbeddingKeys(),
    ]);

  return defer({
    profiles,
    aggregatedGenres,
    aggregatedLanguages,
    embeddingKeys,
    settings,
    recommendations,
    defaultQuantileRankAlbumAssessmentSettings,
  });
};

export const RecommendationPage = () => {
  const {
    profiles,
    aggregatedGenres,
    aggregatedLanguages,
    embeddingKeys,
    settings,
    recommendations,
    defaultQuantileRankAlbumAssessmentSettings,
  } = useLoaderData() as RecommendationSettingsLoaderData;

  return (
    <Grid>
      <Grid.Col
        md={2.75}
        style={{
          borderRight: "1px solid #DDD",
        }}
        sx={{
          "@media (min-width: 1024px)": {
            overflowY: "auto",
            height: "calc(100vh - 55px)",
          },
        }}
        px="md"
      >
        <RecommendationSettings
          profiles={profiles}
          aggregatedGenres={aggregatedGenres}
          aggregatedLanguages={aggregatedLanguages}
          embeddingKeys={embeddingKeys}
          settings={settings}
          defaultQuantileRankAlbumAssessmentSettings={
            defaultQuantileRankAlbumAssessmentSettings
          }
        />
      </Grid.Col>
      <Grid.Col
        md={9.25}
        sx={{
          "@media (min-width: 1024px)": {
            overflowY: "auto",
            height: "calc(100vh - 55px)",
          },
          background: "#eee",
        }}
        px="xs"
      >
        <React.Suspense fallback={<p>Loading recommendations...</p>}>
          <Await resolve={recommendations} errorElement={<ErrorBoundary />}>
            {(recommendations: AlbumRecommendation[] | null) => (
              <Stack spacing="md">
                {recommendations === null ? (
                  <div>Select a profile to get started</div>
                ) : (
                  recommendations.map((r) => (
                    <AlbumRecommendationItem
                      key={r.getAlbum()!.getFileName()}
                      recommendation={r}
                      recommendationMethod={settings?.method}
                    />
                  ))
                )}
              </Stack>
            )}
          </Await>
        </React.Suspense>
      </Grid.Col>
    </Grid>
  );
};
