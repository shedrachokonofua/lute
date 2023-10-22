import { Button, Menu } from "@mantine/core";
import {
  IconAffiliate,
  IconCaretDownFilled,
  IconTools,
  IconTrashX,
} from "@tabler/icons-react";
import { Link, useSubmit } from "react-router-dom";
import { Profile } from "../../../proto/lute_pb";

export const ProfileDetailsMenu = ({ profile }: { profile: Profile }) => {
  const submit = useSubmit();

  return (
    <Menu shadow="md" width={200} position="bottom-end" withArrow>
      <Menu.Target>
        <Button
          variant="outline"
          leftIcon={<IconTools size={18} />}
          rightIcon={<IconCaretDownFilled size={16} />}
        >
          Options
        </Button>
      </Menu.Target>

      <Menu.Dropdown>
        <Menu.Item
          icon={<IconAffiliate size={14} />}
          component={Link}
          to={`/recommendations?profileId=` + profile.getId()}
        >
          Recommendations
        </Menu.Item>
        <Menu.Divider />
        <Menu.Item
          color="red"
          icon={<IconTrashX size={14} />}
          onClick={() => {
            if (
              confirm(
                `Are you sure you want to delete "${profile.getName()}"? This action is irreversible.`,
              )
            ) {
              submit(
                {
                  intent: "delete-profile",
                  "revalidate-remote-context": "true",
                },
                {
                  method: "delete",
                  action: `/profiles/${profile.getId()}`,
                },
              );
            }
          }}
        >
          Delete Profile
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
  );
};
