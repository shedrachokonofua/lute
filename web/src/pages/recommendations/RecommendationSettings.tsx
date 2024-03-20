import { Button, NumberInput, Select, Stack, Title } from "@mantine/core";
import { useState } from "react";
import { Form } from "react-router-dom";
import { CollapsibleSection } from "../../components";
import { AlbumSearchFilters } from "../../components/AlbumSearchFilters";
import { EmbeddingSimilaritySettings } from "../../components/EmbeddingSimilaritySettings";
import { QuantileRankAlbumAssessmentSettings } from "../../proto/lute_pb";
import { useRemoteContext } from "../../remote-context";
import { QuantileRankSettings } from "./QuantileRankSettings";
import {
  RecommendationSettingsForm,
  RecommendationSettingsFormName,
} from "./types";

export const RecommendationSettings = ({
  settings,
  defaultQuantileRankAlbumAssessmentSettings,
}: {
  settings: RecommendationSettingsForm | null;
  defaultQuantileRankAlbumAssessmentSettings: QuantileRankAlbumAssessmentSettings;
}) => {
  const { embeddingKeys, profiles } = useRemoteContext();
  const [currentMethod, setCurrentMethod] = useState<string>(
    settings?.method || "quantile-ranking",
  );
  const profileOptions = profiles.map((profile) => ({
    label: profile.getName(),
    value: profile.getId(),
  }));

  return (
    <Stack spacing="lg">
      <Title order={4}>Settings</Title>
      <Form role="search">
        <Stack spacing="xl">
          <Stack spacing="sm">
            <Select
              label="Profile"
              data={profileOptions}
              placeholder="Select Profile"
              name={RecommendationSettingsFormName.ProfileId}
              defaultValue={settings?.profileId}
              variant="filled"
            />
            <NumberInput
              label="Recommendations Count"
              placeholder="Recommendations Count"
              min={1}
              max={100}
              name={RecommendationSettingsFormName.Count}
              defaultValue={settings?.recommendationSettings?.count || 40}
              variant="filled"
            />
          </Stack>
          <CollapsibleSection title="Filters">
            <AlbumSearchFilters filters={settings?.recommendationSettings} />
          </CollapsibleSection>
          <Stack spacing="sm">
            <Select
              label="Method"
              data={[
                { label: "Quantile Ranking", value: "quantile-ranking" },
                {
                  label: "Embedding Similarity",
                  value: "embedding-similarity",
                },
              ]}
              value={currentMethod}
              onChange={(value) => {
                if (value) {
                  setCurrentMethod(value);
                }
              }}
              placeholder="Select Method"
              name={RecommendationSettingsFormName.Method}
              variant="filled"
            />
            <CollapsibleSection title="Method Settings">
              {currentMethod === "quantile-ranking" && (
                <QuantileRankSettings
                  settings={settings}
                  defaultQuantileRankAlbumAssessmentSettings={
                    defaultQuantileRankAlbumAssessmentSettings
                  }
                />
              )}
              {currentMethod === "embedding-similarity" && (
                <EmbeddingSimilaritySettings
                  defaultEmbeddingKey={
                    settings?.assessmentSettings?.embeddingSimilarity
                      ?.embeddingKey
                  }
                />
              )}
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
