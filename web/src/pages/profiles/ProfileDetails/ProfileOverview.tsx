import { Box, Group, Stack, Text } from "@mantine/core";
import { ReactNode } from "react";
import { ItemWithFactor, ProfileSummary } from "../../../proto/lute_pb";
import { ProfileDetailsCard } from "./ProfileDetailsCard";

const OverviewItem = ({
  label,
  value,
}: {
  label: string;
  value: ReactNode;
}) => {
  return (
    <div>
      <Text size="sm" weight="bold">
        {label}
      </Text>
      <Text size="sm">{value}</Text>
    </div>
  );
};

const getTopItems = (list: ItemWithFactor[], count: number) => {
  return list
    .sort((a, b) => {
      return b.getFactor() - a.getFactor();
    })
    .slice(0, count)
    .map((item) => item.getItem());
};

export const ProfileOverview = ({
  profileSummary,
}: {
  profileSummary: ProfileSummary;
}) => {
  const N = 8;
  const topArtists = getTopItems(profileSummary.getArtistsList(), N);
  const topPrimaryGenres = getTopItems(
    profileSummary.getPrimaryGenresList(),
    N,
  );
  const topSecondaryGenres = getTopItems(
    profileSummary.getSecondaryGenresList(),
    N,
  );
  const topDescriptors = getTopItems(profileSummary.getDescriptorsList(), N);

  return (
    <ProfileDetailsCard label="Overview">
      <Box pt="sm">
        <Stack spacing="sm">
          <Group grow>
            <OverviewItem
              label="Average Rating"
              value={profileSummary.getAverageRating().toFixed(2)}
            />
            <OverviewItem
              label="Median Year"
              value={profileSummary.getMedianYear()}
            />
          </Group>
          <OverviewItem label="Top Artists" value={topArtists.join(", ")} />
          <OverviewItem
            label="Top Primary Genres"
            value={topPrimaryGenres.join(", ")}
          />
          <OverviewItem
            label="Top Secondary Genres"
            value={topSecondaryGenres.join(", ")}
          />
          <OverviewItem
            label="Top Descriptors"
            value={topDescriptors.join(", ")}
          />
        </Stack>
      </Box>
    </ProfileDetailsCard>
  );
};
