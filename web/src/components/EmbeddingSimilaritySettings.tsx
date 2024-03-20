import { Select, Stack } from "@mantine/core";
import { RecommendationSettingsFormName } from "../pages/recommendations/types";
import { useRemoteContext } from "../remote-context";

export const EmbeddingSimilaritySettings = ({
  defaultEmbeddingKey,
}: {
  defaultEmbeddingKey?: string;
}) => {
  const { embeddingKeys } = useRemoteContext();

  return (
    <Stack spacing="sm">
      <Select
        label="Embedding Key"
        placeholder="Embedding Key"
        data={embeddingKeys.map((key) => ({ label: key, value: key }))}
        name={RecommendationSettingsFormName.EmbeddingSimilarityEmbeddingKey}
        defaultValue={defaultEmbeddingKey ?? embeddingKeys[0]}
        variant="filled"
      />
    </Stack>
  );
};
