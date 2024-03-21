import { Select } from "@mantine/core";
import { FormName } from "../forms";
import { useRemoteContext } from "../remote-context";

export const EmbeddingSimilaritySettings = ({
  defaultEmbeddingKey,
}: {
  defaultEmbeddingKey?: string;
}) => {
  const { embeddingKeys } = useRemoteContext();

  return (
    <Select
      label="Embedding Key"
      placeholder="Embedding Key"
      data={embeddingKeys.map((key) => ({ label: key, value: key }))}
      name={FormName.EmbeddingSimilarityEmbeddingKey}
      defaultValue={defaultEmbeddingKey ?? embeddingKeys[0]}
      variant="filled"
    />
  );
};
