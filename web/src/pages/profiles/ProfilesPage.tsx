import { Box, Button, Container, Group } from "@mantine/core";
import { IconPlus } from "@tabler/icons-react";
import { ActionFunction, Link, Outlet, useMatches } from "react-router-dom";
import { useRemoteContext } from "../../remote-context";
import { ProfileDetailsMenu } from "./ProfileDetails";
import { ProfileSelect } from "./ProfilesSelect";

export const profilePageAction: ActionFunction = () => {
  return null;
};

const useActiveProfile = () => {
  const { profiles } = useRemoteContext();
  const matches = useMatches();
  const id = matches[matches.length - 1]?.params.id;
  if (!id) {
    return undefined;
  }
  return profiles.find((p) => p.getId() === id);
};

export const ProfilesPage = () => {
  const activeProfile = useActiveProfile();

  return (
    <div
      style={{
        background: "#EEE",
        minHeight: "100%",
      }}
    >
      <Box
        style={{
          background: "#FFF",
          borderBottom: "1px solid #DDD",
        }}
        py="md"
      >
        <Container size="lg">
          <Group position="apart">
            <Group>
              <Button
                component={Link}
                to="/profiles/new"
                leftIcon={<IconPlus size={16} />}
              >
                Create Profile
              </Button>
              <div>
                <ProfileSelect id={activeProfile?.getId()} />
              </div>
            </Group>
            <div>
              {activeProfile && <ProfileDetailsMenu profile={activeProfile} />}
            </div>
          </Group>
        </Container>
      </Box>
      <Container size="lg" py="md">
        <Outlet />
      </Container>
    </div>
  );
};
