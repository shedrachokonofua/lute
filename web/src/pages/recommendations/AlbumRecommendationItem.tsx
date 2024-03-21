import { Popover, Text, Title } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { AlbumCard } from "../../components";
import { AlbumAssessment, AlbumRecommendation } from "../../proto/lute_pb";
import { RecommendationMethod } from "./types";

interface AlbumRecommendationItemProps {
  recommendation: AlbumRecommendation;
  recommendationMethod?: RecommendationMethod;
}

const Assessment = ({ assessment }: { assessment: AlbumAssessment }) => {
  const [opened, { close, open }] = useDisclosure(false);
  const hasMetadata = assessment.getMetadataMap().getLength() > 0;

  const score = (
    <Title
      order={4}
      onMouseEnter={() => {
        if (hasMetadata) {
          open();
        }
      }}
      onMouseLeave={() => {
        close();
      }}
      style={{
        cursor: "pointer",
      }}
    >
      {(assessment.getScore() * 100).toFixed(0)}%
    </Title>
  );

  return hasMetadata ? (
    <Popover
      width={250}
      position="left-start"
      withArrow
      shadow="md"
      opened={opened}
    >
      <Popover.Target>{score}</Popover.Target>
      <Popover.Dropdown sx={{ pointerEvents: "none" }}>
        <Text size="sm">
          <b>Score</b>: {(assessment.getScore() * 100).toFixed(2)}%
        </Text>
        {assessment
          .getMetadataMap()
          .getEntryList()
          .map(([key, value]) => (
            <Text size="sm" key={key}>
              <b>{key}</b>: {(Number(value) * 100).toFixed(2)}%
            </Text>
          ))}
      </Popover.Dropdown>
    </Popover>
  ) : (
    score
  );
};

export const AlbumRecommendationItem = ({
  recommendation,
  recommendationMethod,
}: AlbumRecommendationItemProps) => {
  const album = recommendation.getAlbum()!;
  const assessment = recommendation.getAssessment()!;

  return (
    <AlbumCard
      album={album}
      assessment={
        recommendationMethod === "quantile-ranking" ? (
          <Assessment assessment={assessment} />
        ) : undefined
      }
    />
  );
};
