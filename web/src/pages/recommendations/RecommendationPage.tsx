import { Button, Flex, Stack, Text } from "@mantine/core";
import React from "react";
import {
  Await,
  Link,
  LoaderFunctionArgs,
  defer,
  useLoaderData,
  useRouteError,
} from "react-router-dom";
import {
  getAlbumRecommendations,
  getDefaultQuantileRankAlbumAssessmentSettings,
} from "../../client";
import { TwoColumnLayout } from "../../components";
import { Page } from "../../components/Page";
import {
  FormName,
  coerceToUndefined,
  parseAlbumSearchFiltersForm,
  toNumber,
} from "../../forms";
import {
  AlbumRecommendation,
  QuantileRankAlbumAssessmentSettings,
} from "../../proto/lute_pb";
import { AlbumRecommendationItem } from "./AlbumRecommendationItem";
import { RecommendationSettings } from "./RecommendationSettings";
import { RecommendationMethod, RecommendationSettingsForm } from "./types";

const ErrorBoundary = () => {
  const error = useRouteError();
  console.error(error);
  return <div>Dang! Something went wrong.</div>;
};

interface RecommendationSettingsLoaderData {
  settings: RecommendationSettingsForm | null;
  recommendations: AlbumRecommendation[] | null;
  defaultQuantileRankAlbumAssessmentSettings: QuantileRankAlbumAssessmentSettings;
}

export const recommendationPageLoader = async ({
  request,
}: LoaderFunctionArgs) => {
  const url = new URL(request.url);
  const profileId = url.searchParams.get(FormName.ProfileId);
  const assessmentMethod =
    url.searchParams.get(FormName.Method) || "quantile-ranking";

  const assessmentSettings =
    assessmentMethod === "quantile-ranking"
      ? coerceToUndefined({
          quantileRanking: {
            primaryGenresWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  FormName.QuantileRankingPrimaryGenresWeight,
                ),
              ),
            ),
            secondaryGenresWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  FormName.QuantileRankingSecondaryGenresWeight,
                ),
              ),
            ),
            descriptorWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(FormName.QuantileRankingDescriptorWeight),
              ),
            ),
            ratingWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(FormName.QuantileRankingRatingWeight),
              ),
            ),
            ratingCountWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(FormName.QuantileRankingRatingCountWeight),
              ),
            ),
            descriptorCountWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(
                  FormName.QuantileRankingDescriptorCountWeight,
                ),
              ),
            ),
            creditTagWeight: toNumber(
              coerceToUndefined(
                url.searchParams.get(FormName.QuantileRankingCreditTagWeight),
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
              url.searchParams.get(FormName.EmbeddingSimilarityEmbeddingKey),
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
        recommendationSettings: parseAlbumSearchFiltersForm(url),
        assessmentSettings,
      }
    : null;

  const recommendations = settings ? getAlbumRecommendations(settings) : null;
  const defaultQuantileRankAlbumAssessmentSettings =
    await getDefaultQuantileRankAlbumAssessmentSettings();

  return defer({
    settings,
    recommendations,
    defaultQuantileRankAlbumAssessmentSettings,
  });
};

export const Component = () => {
  const {
    settings,
    recommendations,
    defaultQuantileRankAlbumAssessmentSettings,
  } = useLoaderData() as RecommendationSettingsLoaderData;

  let playlistPreviewUrl = new URL(window.location.href);
  playlistPreviewUrl.pathname = playlistPreviewUrl.pathname + "/playlist";

  return (
    <Page>
      <TwoColumnLayout
        left={
          <div>
            <RecommendationSettings
              settings={settings}
              defaultQuantileRankAlbumAssessmentSettings={
                defaultQuantileRankAlbumAssessmentSettings
              }
            />
          </div>
        }
        right={
          <React.Suspense fallback={<Text>Loading recommendations...</Text>}>
            <Await resolve={recommendations} errorElement={<ErrorBoundary />}>
              {(recommendations: AlbumRecommendation[] | null) => {
                return (
                  <div>
                    {recommendations === null ? (
                      <Text>Select a profile to get started</Text>
                    ) : (
                      <div style={{ position: "relative" }}>
                        <Flex
                          gap="md"
                          py="xs"
                          px="xs"
                          justify="end"
                          style={{
                            background: "rgba(30, 87, 174, 0.3)",
                            boxShadow: "0 2px 4px rgba(0, 0, 0, 0.1)",
                            position: "sticky",
                            top: 0,
                            zIndex: 1000,
                          }}
                        >
                          <Button
                            component={Link}
                            size="compact-sm"
                            radius={2}
                            variant="white"
                            to={playlistPreviewUrl.toString()}
                          >
                            Generate Spotify Playlist
                          </Button>
                        </Flex>
                        <Stack gap="md" px="xs" py="sm">
                          {recommendations.map((r) => (
                            <AlbumRecommendationItem
                              key={r.getAlbum()!.getFileName()}
                              recommendation={r}
                              recommendationMethod={settings?.method}
                            />
                          ))}
                        </Stack>
                      </div>
                    )}
                  </div>
                );
              }}
            </Await>
          </React.Suspense>
        }
      />
    </Page>
  );
};

Component.displayName = "RecommendationPage";
