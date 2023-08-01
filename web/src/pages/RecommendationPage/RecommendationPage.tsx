import { Badge, Button, Grid, Stack, Text, Title } from "@mantine/core";
import React from "react";
import {
  RecommendationMethod,
  RecommendationSettings,
  RecommendationSettingsForm,
  RecommendationSettingsFormName,
} from "./RecommendationSettings";
import {
  getAggregatedGenres,
  getAlbumRecommendations,
  getAllProfiles,
} from "../../client";
import {
  Await,
  LoaderFunctionArgs,
  defer,
  useLoaderData,
} from "react-router-dom";
import {
  AlbumRecommendation,
  GenreAggregate,
  Profile,
} from "../../proto/lute_pb";
import queryString from "query-string";

interface RecommendationQueryValue {
  status: "pending" | "success" | "error";
  message: string;
  recommendations: AlbumRecommendation[];
}

const coerceToUndefined = <
  T extends string | Record<string, unknown> | null | undefined
>(
  value: T
): T | undefined => {
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

export const recommendationPageLoader = async ({
  request,
}: LoaderFunctionArgs) => {
  const [profiles, aggregatedGenres] = await Promise.all([
    getAllProfiles(),
    getAggregatedGenres(),
  ]);
  const url = new URL(request.url);
  const profileId = url.searchParams.get(
    RecommendationSettingsFormName.ProfileId
  );
  const assessmentMethod = url.searchParams.get(
    RecommendationSettingsFormName.Method
  );
  const assessmentSettings =
    assessmentMethod === "quantile-ranking"
      ? coerceToUndefined({
          quantileRanking: {
            primaryGenresWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingPrimaryGenresWeight
                )
              )
            ),
            secondaryGenresWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingSecondaryGenresWeight
                )
              )
            ),
            descriptorWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingDescriptorWeight
                )
              )
            ),
            ratingWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingRatingWeight
                )
              )
            ),
            ratingCountWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  RecommendationSettingsFormName.QuantileRankingRatingCountWeight
                )
              )
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
              url.searchParams.get(RecommendationSettingsFormName.Count)
            )
          ),
          includePrimaryGenres: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.IncludePrimaryGenres
            )
          )?.split(","),
          excludePrimaryGenres: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.ExcludePrimaryGenres
            )
          )?.split(","),
          includeSecondaryGenres: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.IncludeSecondaryGenres
            )
          )?.split(","),
          excludeSecondaryGenres: coerceToUndefined(
            url.searchParams.get(
              RecommendationSettingsFormName.ExcludeSecondaryGenres
            )
          )?.split(","),
        }),
        assessmentSettings,
      }
    : null;

  const recommendations = settings ? getAlbumRecommendations(settings) : null;

  return defer({
    profiles,
    aggregatedGenres,
    settings,
    recommendations,
  });
};

export const RecommendationPage = () => {
  const { profiles, aggregatedGenres, settings, recommendations } =
    useLoaderData() as {
      profiles: Profile[];
      aggregatedGenres: GenreAggregate[];
      settings: RecommendationSettingsForm;
      recommendations: AlbumRecommendation[] | null;
    };

  return (
    <Grid>
      <Grid.Col
        md={2.75}
        style={{
          background: "#FAFDFF",
          borderRight: "1px solid rgb(200, 225, 235)",
          boxShadow:
            "inset -4px 0 10px -5px rgba(0, 0, 0, 0.15), inset 4px 0 10px -5px rgba(0, 0, 0, 0.15)",
          overflowY: "auto",
          height: "calc(100vh - 55px)",
        }}
        px="md"
      >
        <RecommendationSettings
          profiles={profiles}
          aggregatedGenres={aggregatedGenres}
          settings={settings}
        />
      </Grid.Col>
      <Grid.Col
        md={9.25}
        style={{
          overflowY: "auto",
          height: "calc(100vh - 55px)",
        }}
        px="xs"
      >
        <React.Suspense fallback={<p>Loading recommendations...</p>}>
          <Await
            resolve={recommendations}
            errorElement={<p>Error loading recommendations!</p>}
          >
            {(recommendations: AlbumRecommendation[] | null) => (
              <Stack spacing="xl">
                {recommendations === null ? (
                  <div>Select a profile to get started</div>
                ) : (
                  recommendations.map((r) => {
                    const album = r.getAlbum()!;

                    return (
                      <div>
                        <Title order={3}>
                          <a
                            href={`https://rateyourmusic.com/${album.getFileName()}`}
                            target="_blank"
                            style={{ textDecoration: "none" }}
                          >
                            {album.getName()}
                          </a>
                        </Title>
                        <Title order={5}>
                          {album
                            .getArtistsList()
                            .map((a) => a.getName())
                            .join(", ")}
                        </Title>
                        <div>
                          <Badge
                            variant="gradient"
                            gradient={{ from: "teal", to: "blue", deg: 60 }}
                          >
                            {album.getRating().toFixed(2)}/5
                          </Badge>
                        </div>
                        <Text weight="semi-bold">
                          {album.getPrimaryGenresList().join(", ")}
                        </Text>
                        <Text size={"md"}>
                          {album.getSecondaryGenresList().join(", ")}
                        </Text>
                        <Text size="sm">
                          {album.getDescriptorsList().join(", ")}
                        </Text>
                      </div>
                    );
                  })
                )}
              </Stack>
            )}
          </Await>
        </React.Suspense>
      </Grid.Col>
    </Grid>
  );
};
