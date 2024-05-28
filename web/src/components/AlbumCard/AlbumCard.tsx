import { Anchor, Badge, Box, Card, Flex, Text } from "@mantine/core";
import { Album } from "../../proto/lute_pb";
import classes from "./AlbumCard.module.css";

export const AlbumCard = ({
  album,
  assessment,
  actions,
}: {
  album: Album;
  assessment?: React.ReactNode;
  actions?: React.ReactNode;
}) => (
  <Card padding="sm" shadow="xs" withBorder>
    <Flex gap="md" className={classes.content}>
      <Box className={classes.imageBox}>
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
            <Text fw="bold" size="1.25rem">
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

          {assessment}
        </Flex>
        <Text fw="bold">
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
        <Text fw="semi-bold">{album.getPrimaryGenresList().join(", ")}</Text>
        <Text size="md">{album.getSecondaryGenresList().join(", ")}</Text>
        <Text size="sm">{album.getDescriptorsList().join(", ")}</Text>
        {actions && <Box mt="1rem">{actions}</Box>}
      </div>
    </Flex>
  </Card>
);
