import { Select, Stack } from "@mantine/core";
import {
  RecommendationSettingsForm,
  RecommendationSettingsFormName,
} from "./types";

export const EmbeddingSimilaritySettings = ({
  settings,
  embbedingKeys,
}: {
  settings: RecommendationSettingsForm | null;
  embbedingKeys: string[];
}) => (
  <Stack spacing="sm">
    <Select
      label="Embedding Key"
      placeholder="Embedding Key"
      data={embbedingKeys.map((key) => ({ label: key, value: key }))}
      name={RecommendationSettingsFormName.EmbeddingSimilarityEmbeddingKey}
      defaultValue={
        settings?.assessmentSettings?.embeddingSimilarity?.embeddingKey ??
        embbedingKeys[0]
      }
      variant="filled"
    />
  </Stack>
);
