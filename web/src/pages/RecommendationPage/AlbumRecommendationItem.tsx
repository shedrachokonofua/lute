import { Badge, Flex, Text, Title } from "@mantine/core";
import { AlbumRecommendation } from "../../proto/lute_pb";

interface AlbumRecommendationItemProps {
  recommendation: AlbumRecommendation;
}

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
        <Title order={4}>{(assessment.getScore() * 100).toFixed(0)}%</Title>
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
