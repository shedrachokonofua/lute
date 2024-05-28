import { Button, NumberInput, Select, Stack, Title } from "@mantine/core";
import { useState } from "react";
import { Form } from "react-router-dom";
import {
  AlbumSearchFilters,
  CollapsibleSection,
  EmbeddingSimilaritySettings,
} from "../../components";
import { FormName } from "../../forms";
import { QuantileRankAlbumAssessmentSettings } from "../../proto/lute_pb";
import { useRemoteContext } from "../../remote-context";
import { QuantileRankSettings } from "./QuantileRankSettings";
import { RecommendationSettingsForm } from "./types";

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
