import { Box, Button, Flex, Text } from "@mantine/core";
import React from "react";
import {
  Await,
  Link,
  LoaderFunctionArgs,
  defer,
  useLoaderData,
  useRouteError,
} from "react-router-dom";
import { Spotify } from "react-spotify-embed";
import {
  draftSpotifyPlaylist,
  getDefaultQuantileRankAlbumAssessmentSettings,
} from "../../client";
import { TwoColumnLayout } from "../../components";
import {
  FormName,
  coerceToUndefined,
  parseAlbumSearchFiltersForm,
  toNumber,
} from "../../forms";
import {
  QuantileRankAlbumAssessmentSettings,
  SpotifyTrackReference,
} from "../../proto/lute_pb";
import { RecommendationSettings } from "./RecommendationSettings";
import { RecommendationMethod, RecommendationSettingsForm } from "./types";

const ErrorBoundary = () => {
  const error = useRouteError();
  console.error(error);
  return <div>Dang! Something went wrong.</div>;
};

interface PlaylistPreviewSettingsLoaderData {
  settings: RecommendationSettingsForm | null;
  tracks: SpotifyTrackReference[] | null;
  defaultQuantileRankAlbumAssessmentSettings: QuantileRankAlbumAssessmentSettings;
}

export const playlistPreviewPageLoader = async ({
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

  const tracks = settings ? draftSpotifyPlaylist(settings) : null;
  const defaultQuantileRankAlbumAssessmentSettings =
    await getDefaultQuantileRankAlbumAssessmentSettings();

  return defer({
    settings,
    tracks,
    defaultQuantileRankAlbumAssessmentSettings,
  });
};

export const PlaylistPreviewPage = () => {
  const { settings, tracks, defaultQuantileRankAlbumAssessmentSettings } =
    useLoaderData() as PlaylistPreviewSettingsLoaderData;

  let playlistPreviewUrl = new URL(window.location.href);
  playlistPreviewUrl.pathname = playlistPreviewUrl.pathname.replace(
    "/playlist",
    "",
  );

  return (
    <div>
      <Flex
        gap="md"
        py="xs"
        px="xs"
        justify="end"
        sx={{
          background: "#000",
          borderBottom: "1px solid #DDD",
        }}
      >
        <Button
          variant="white"
          component={Link}
          compact
          radius={2}
          to={playlistPreviewUrl.toString()}
        >
          Back
        </Button>
      </Flex>
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
            <Await resolve={tracks} errorElement={<ErrorBoundary />}>
              {(tracks: SpotifyTrackReference[] | null) => {
                console.log(tracks?.map((t) => t.getSpotifyId()));
                return (
                  <Box px="xs">
                    <Flex gap="md" wrap="wrap">
                      {tracks === null ? (
                        <Text>Select a profile to get started</Text>
                      ) : (
                        tracks.map((t) => (
                          <Spotify
                            link={`https://open.spotify.com/track/${t
                              .getSpotifyId()
                              .replace("spotify:track:", "")}`}
                            key={t.getSpotifyId()}
                          />
                        ))
                      )}
                    </Flex>
                  </Box>
                );
              }}
            </Await>
          </React.Suspense>
        }
      />
    </div>
  );
};
