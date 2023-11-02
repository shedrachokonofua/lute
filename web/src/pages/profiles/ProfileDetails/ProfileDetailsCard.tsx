import { ActionIcon, Card, Menu, Text } from "@mantine/core";
import { IconMenu2 } from "@tabler/icons-react";

export const ProfileDetailsCard = ({
  label,
  children,
  footer,
  dropdownMenu,
}: {
  label: string;
  children?: React.ReactNode;
  footer?: React.ReactNode;
  dropdownMenu?: React.ReactNode;
}) => (
  <Card
    withBorder
    shadow="xs"
    style={
      dropdownMenu
        ? {
            overflow: "visible",
          }
        : {}
    }
  >
    <Card.Section withBorder inheritPadding py="xs">
      <div style={{ display: "flex" }}>
        <div style={{ flex: 1 }}>
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
        </div>
        {dropdownMenu && (
          <div>
            <Menu shadow="md" position="bottom-start">
              <Menu.Target>
                <ActionIcon size={14}>
                  <IconMenu2 />
                </ActionIcon>
              </Menu.Target>

              <Menu.Dropdown>{dropdownMenu}</Menu.Dropdown>
            </Menu>
          </div>
        )}
      </div>
    </Card.Section>
    {children}
    {footer && (
      <Card.Section withBorder inheritPadding py="xs">
        {footer}
      </Card.Section>
    )}
  </Card>
);
