import { Select } from "@mantine/core";
import { useNavigate, useNavigation } from "react-router-dom";
import { useRemoteContext } from "../../remote-context";

export const ProfileSelect = ({ id }: { id?: string }) => {
  const { profiles } = useRemoteContext();
  const navigate = useNavigate();
  const isNavigating = useNavigation().state !== "idle";

  return (
    <Select
      searchable
      size="sm"
      label="Select a profile:"
      placeholder="Select a profile"
      value={id || null}
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
  );
};
