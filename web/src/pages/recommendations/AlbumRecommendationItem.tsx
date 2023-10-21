import {
  Anchor,
  Badge,
  Box,
  Card,
  Flex,
  Popover,
  Text,
  Title,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
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
    <Card padding="sm" shadow="xs" withBorder>
      <Flex
        gap="md"
        sx={{
          "@media (max-width: 1024px)": {
            flexDirection: "column",
          },
        }}
      >
        <Box
          sx={{
            "@media (max-width: 1024px)": {
              width: "100%",
            },
            width: 220,
          }}
        >
          <img
            src={album.getCoverImageUrl()}
            alt={album.getName()}
            style={{
              height: "auto",
              width: "100%",
            }}
          />
        </Box>
        <div
          style={{
            flex: 1,
          }}
        >
          <Flex justify="space-between">
            <Flex align="center" gap="0.5rem">
              <Text weight="bold" size="1.25rem">
                <Anchor
                  href={`https://rateyourmusic.com/${album.getFileName()}`}
                  target="_blank"
                >
                  {album.getName()}
                </Anchor>
              </Text>
              <Badge
                variant="gradient"
                gradient={{ from: "teal", to: "blue", deg: 60 }}
              >
                {album.getRating().toFixed(2)}/5
              </Badge>
            </Flex>

            {recommendationMethod === "quantile-ranking" && (
              <Assessment assessment={assessment} />
            )}
          </Flex>
          <Text weight="bold">
            {album
              .getArtistsList()
              .map((a) => a.getName())
              .join(", ")}
          </Text>
          <div>
            <Text size="sm" color="#333">
              {album.getReleaseDate()}
            </Text>
          </div>
          <Text weight="semi-bold">
            {album.getPrimaryGenresList().join(", ")}
          </Text>
          <Text size="md">{album.getSecondaryGenresList().join(", ")}</Text>
          <Text size="sm">{album.getDescriptorsList().join(", ")}</Text>
        </div>
      </Flex>
    </Card>
  );
};
