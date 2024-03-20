import { Anchor, Badge, Box, Card, Flex, Text } from "@mantine/core";
import { Album } from "../proto/lute_pb";

export const AlbumCard = ({
  album,
  children,
}: {
  album: Album;
  children?: React.ReactNode;
}) => (
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

          {children}
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
