import { Badge, Flex, Popover, Text, Title } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { AlbumAssessment, AlbumRecommendation } from "../../proto/lute_pb";

interface AlbumRecommendationItemProps {
  recommendation: AlbumRecommendation;
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
}: AlbumRecommendationItemProps) => {
  const album = recommendation.getAlbum()!;
  const assessment = recommendation.getAssessment()!;

  return (
    <div>
      <Flex justify="space-between">
        <Title order={3}>
          <a
            href={`https://rateyourmusic.com/${album.getFileName()}`}
            target="_blank"
            style={{ textDecoration: "none" }}
          >
            {album.getName()}
          </a>
        </Title>
        <Assessment assessment={assessment} />
      </Flex>
      <Title order={5}>
        {album
          .getArtistsList()
          .map((a) => a.getName())
          .join(", ")}
      </Title>
      <div>
        <Badge
          variant="gradient"
          gradient={{ from: "teal", to: "blue", deg: 60 }}
        >
          {album.getRating().toFixed(2)}/5
        </Badge>
      </div>
      <Text weight="semi-bold">{album.getPrimaryGenresList().join(", ")}</Text>
      <Text size="md">{album.getSecondaryGenresList().join(", ")}</Text>
      <Text size="sm">{album.getDescriptorsList().join(", ")}</Text>
    </div>
  );
};
