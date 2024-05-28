import { Box, Button, Container, Group } from "@mantine/core";
import { IconPlus } from "@tabler/icons-react";
import { ActionFunction, Link, Outlet, useMatches } from "react-router-dom";
import { Page } from "../../components/Page";
import { useRemoteContext } from "../../remote-context";
import { ProfileDetailsMenu } from "./ProfileDetails/ProfileDetailsMenu";
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

export const Component = () => {
  const activeProfile = useActiveProfile();

  return (
    <Page
      style={{
        background: "#EEE",
      }}
    >
      <Box
        style={{
          background: "#FFF",
          borderBottom: "1px solid #DDD",
        }}
        py="md"
      >
        <Container size="xl">
          <Group justify="space-between">
            <Group>
              <Button
                component={Link}
                to="/profiles/new"
                leftSection={<IconPlus size={16} />}
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
      <Container size="xl" py="md">
        <Outlet />
      </Container>
    </Page>
  );
};

Component.displayName = "ProfilesPage";
