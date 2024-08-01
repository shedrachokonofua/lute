import { Button, NumberInput, Select, Stack, Title } from "@mantine/core";
import { useState } from "react";
import { Form } from "react-router-dom";
import { z } from "zod";
import {
  AlbumSearchFilters,
  CollapsibleSection,
  EmbeddingSimilaritySettings,
} from "../../components";
import {
  FormName,
  coerceToUndefined,
  parseAlbumSearchFiltersForm,
} from "../../forms";
import { QuantileRankAlbumAssessmentSettings } from "../../proto/lute_pb";
import { useRemoteContext } from "../../remote-context";
import { QuantileRankSettings } from "./QuantileRankSettings";
import { RecommendationMethod, RecommendationSettingsForm } from "./types";

const QuantileRankingSettingsSchema = z.object({
  primaryGenresWeight: z.coerce.number().optional(),
  secondaryGenresWeight: z.coerce.number().optional(),
  descriptorWeight: z.coerce.number().optional(),
  ratingWeight: z.coerce.number().optional(),
  ratingCountWeight: z.coerce.number().optional(),
  descriptorCountWeight: z.coerce.number().optional(),
  creditTagWeight: z.coerce.number().optional(),
});

const EmbeddingSimilaritySettingsSchema = z.object({
  embeddingKey: z.string().optional(),
});

const getQuantileRankingSettings = (url: URL) => {
  return QuantileRankingSettingsSchema.parse({
    primaryGenresWeight: coerceToUndefined(
      url.searchParams.get(FormName.QuantileRankingPrimaryGenresWeight),
    ),
    secondaryGenresWeight: coerceToUndefined(
      url.searchParams.get(FormName.QuantileRankingSecondaryGenresWeight),
    ),
    descriptorWeight: coerceToUndefined(
      url.searchParams.get(FormName.QuantileRankingDescriptorWeight),
    ),
    ratingWeight: coerceToUndefined(
      url.searchParams.get(FormName.QuantileRankingRatingWeight),
    ),
    ratingCountWeight: coerceToUndefined(
      url.searchParams.get(FormName.QuantileRankingRatingCountWeight),
    ),
    descriptorCountWeight: coerceToUndefined(
      url.searchParams.get(FormName.QuantileRankingDescriptorCountWeight),
    ),
    creditTagWeight: coerceToUndefined(
      url.searchParams.get(FormName.QuantileRankingCreditTagWeight),
    ),
  });
};

const getEmbeddingSimilaritySettings = (url: URL) => {
  return EmbeddingSimilaritySettingsSchema.parse({
    embeddingKey: coerceToUndefined(
      url.searchParams.get(FormName.EmbeddingSimilarityEmbeddingKey),
    ),
  });
};

export const getRecommendationSettingsFromUrl = (url: URL) => {
  const profileId = url.searchParams.get(FormName.ProfileId);
  const assessmentMethod =
    url.searchParams.get(FormName.Method) || "reranked-embedding-similarity";

  const assessmentSettings =
    assessmentMethod === "quantile-ranking"
      ? {
          quantileRanking: getQuantileRankingSettings(url),
          embeddingSimilarity: undefined,
        }
      : assessmentMethod === "embedding-similarity"
      ? {
          quantileRanking: undefined,
          embeddingSimilarity: getEmbeddingSimilaritySettings(url),
        }
      : assessmentMethod === "reranked-embedding-similarity"
      ? {
          quantileRanking: getQuantileRankingSettings(url),
          embeddingSimilarity: getEmbeddingSimilaritySettings(url),
          minEmbeddingCandidateCount: 500,
        }
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

  return settings;
};
export const RecommendationSettings = ({
  settings,
  defaultQuantileRankAlbumAssessmentSettings,
}: {
  settings: RecommendationSettingsForm | null;
  defaultQuantileRankAlbumAssessmentSettings: QuantileRankAlbumAssessmentSettings;
}) => {
  const { profiles } = useRemoteContext();
  const [currentMethod, setCurrentMethod] = useState<string>(
    settings?.method || "quantile-ranking",
  );

  return (
    <Stack gap="lg">
      <Title order={4}>Settings</Title>
      <Form role="search">
        <Stack gap="xl">
          <Stack gap="sm">
            <Select
              label="Profile"
              data={profiles.map((profile) => ({
                label: profile.getName(),
                value: profile.getId(),
              }))}
              placeholder="Select Profile"
              name={FormName.ProfileId}
              defaultValue={settings?.profileId}
              variant="filled"
            />
            <NumberInput
              label="Recommendations Count"
              placeholder="Recommendations Count"
              min={1}
              max={100}
              name={FormName.Count}
              defaultValue={settings?.recommendationSettings?.count || 40}
              variant="filled"
            />
          </Stack>
          <CollapsibleSection title="Filters">
            <AlbumSearchFilters filters={settings?.recommendationSettings} />
          </CollapsibleSection>
          <Stack gap="sm">
            <Select
              label="Method"
              data={[
                { label: "Quantile Ranking", value: "quantile-ranking" },
                {
                  label: "Embedding Similarity",
                  value: "embedding-similarity",
                },
                {
                  label: "Reranked Embedding Similarity",
                  value: "reranked-embedding-similarity",
                },
              ]}
              value={currentMethod}
              onChange={(value) => {
                if (value) {
                  setCurrentMethod(value);
                }
              }}
              placeholder="Select Method"
              name={FormName.Method}
              variant="filled"
            />
            <CollapsibleSection title="Method Settings">
              <Stack gap="sm">
                {(currentMethod === "embedding-similarity" ||
                  currentMethod === "reranked-embedding-similarity") && (
                  <EmbeddingSimilaritySettings
                    defaultEmbeddingKey={
                      settings?.assessmentSettings?.embeddingSimilarity
                        ?.embeddingKey
                    }
                  />
                )}
                {(currentMethod === "quantile-ranking" ||
                  currentMethod === "reranked-embedding-similarity") && (
                  <QuantileRankSettings
                    settings={settings}
                    defaultQuantileRankAlbumAssessmentSettings={
                      defaultQuantileRankAlbumAssessmentSettings
                    }
                  />
                )}
              </Stack>
            </CollapsibleSection>
          </Stack>
          <div>
            <Button type="submit" fullWidth>
              Submit
            </Button>
          </div>
        </Stack>
      </Form>
    </Stack>
  );
};
