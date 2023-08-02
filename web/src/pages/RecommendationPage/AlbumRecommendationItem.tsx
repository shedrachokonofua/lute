import React from "react";
import { AlbumRecommendation } from "../../proto/lute_pb";
import { Title, Badge, Text } from "@mantine/core";

interface AlbumRecommendationItemProps {
  recommendation: AlbumRecommendation;
}

export const AlbumRecommendationItem = ({
  recommendation,
}: AlbumRecommendationItemProps) => {
  const album = recommendation.getAlbum()!;

  return (
    <div>
      <Title order={3}>
        <a
          href={`https://rateyourmusic.com/${album.getFileName()}`}
          target="_blank"
          style={{ textDecoration: "none" }}
        >
          {album.getName()}
        </a>
      </Title>
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
