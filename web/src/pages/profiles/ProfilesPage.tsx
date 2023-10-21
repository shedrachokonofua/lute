import { Box, Button, Container, Group, Select } from "@mantine/core";
import {
  ActionFunction,
  Link,
  Outlet,
  useNavigate,
  useNavigation,
  useParams,
} from "react-router-dom";
import { useRemoteContext } from "../../remote-context";

export const profilePageAction: ActionFunction = () => {
  return null;
};

export const ProfilesPage = () => {
  const { profiles } = useRemoteContext();
  const navigate = useNavigate();
  const { id } = useParams();
  const isNavigating = useNavigation().state !== "idle";

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
            <div>
              <Select
                searchable
                size="sm"
                label="Select a profile:"
                placeholder="Select a profile"
                defaultValue={id}
                disabled={isNavigating}
                data={profiles.map((p) => ({
                  label: p.getName(),
                  value: p.getId(),
                }))}
                styles={{
                  root: {
                    display: "flex",
                    alignItems: "center",
                    gap: "0.5rem",
                  },
                  input: {
                    width: 300,
                  },
                }}
                onChange={(id) => {
                  if (id) {
                    navigate(`/profiles/${id}`);
                  }
                }}
              />
            </div>
            <div>
              <Button component={Link} to="/profiles/new">
                Create Profile
              </Button>
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
