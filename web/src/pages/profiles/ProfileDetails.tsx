import { Card, Grid, Text } from "@mantine/core";
import { LoaderFunctionArgs, useLoaderData } from "react-router-dom";
import { getProfile, getProfileSummary } from "../../client";
import { Profile, ProfileSummary } from "../../proto/lute_pb";

interface ProfileDetailsLoaderData {
  profile: Profile;
  profileSummary: ProfileSummary;
}

export const profileDetailsLoader = async ({ params }: LoaderFunctionArgs) => {
  const id = params.id as string;
  const [profile, profileSummary] = await Promise.all([
    getProfile(id),
    getProfileSummary(id),
  ]);

  if (!profile || !profileSummary) {
    throw new Error("Profile not found");
  }

  return {
    profile,
    profileSummary,
  };
};

const ProfileDetailsCard = ({
  label,
  children,
}: {
  label: string;
  children?: React.ReactNode;
}) => (
  <Card
    withBorder
    style={{
      height: 400,
    }}
    shadow="xs"
  >
    <Card.Section withBorder inheritPadding py="xs">
      <Text
        style={{
          lineHeight: 1,
        }}
        color="dimmed"
        size="sm"
        weight="bold"
      >
        {label}
      </Text>
    </Card.Section>
    <div>{children}</div>
  </Card>
);

export const ProfileDetails = () => {
  const { profile, profileSummary } =
    useLoaderData() as ProfileDetailsLoaderData;

  return (
    <Grid>
      <Grid.Col md={3}>
        <ProfileDetailsCard label="Summary"></ProfileDetailsCard>
      </Grid.Col>
      <Grid.Col md={9}>
        <ProfileDetailsCard
          label={`Albums(${profile.getAlbumsMap().getLength()})`}
        ></ProfileDetailsCard>
      </Grid.Col>
    </Grid>
  );
};
