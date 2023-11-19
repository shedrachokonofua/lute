import {
  ActionIcon,
  Box,
  Card as MantineCard,
  Menu,
  SpacingValue,
  SystemProp,
  Text,
} from "@mantine/core";
import { IconMenu2 } from "@tabler/icons-react";

export const Card = ({
  label,
  children,
  contentPt,
  footer,
  dropdownMenu,
}: {
  label: string;
  children?: React.ReactNode;
  contentPt?: SystemProp<SpacingValue>;
  footer?: React.ReactNode;
  dropdownMenu?: React.ReactNode;
}) => (
  <MantineCard
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
    <MantineCard.Section withBorder inheritPadding py="xs">
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
    </MantineCard.Section>
    <Box pt={contentPt}>{children}</Box>
    {footer && (
      <MantineCard.Section withBorder inheritPadding py="xs">
        {footer}
      </MantineCard.Section>
    )}
  </MantineCard>
);
