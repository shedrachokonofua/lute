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
  AlbumRecommendation,
  QuantileRankAlbumAssessmentSettings,
} from "../../proto/lute_pb";
import { AlbumRecommendationItem } from "./AlbumRecommendationItem";
import {
  RecommendationSettings,
  getRecommendationSettingsFromUrl,
} from "./RecommendationSettings";
import { RecommendationSettingsForm } from "./types";

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
  const settings = getRecommendationSettingsFromUrl(url);
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
                            background: "#FFF",
                            borderBottom: "1px solid #DDD",
                            position: "sticky",
                            top: 0,
                            zIndex: 1000,
                          }}
                        >
                          <Button
                            component={Link}
                            size="compact-sm"
                            radius={2}
                            variant="light"
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
