import {
  ActionIcon,
  Box,
  Card as MantineCard,
  Menu,
  Text,
} from "@mantine/core";
import { IconMenu2 } from "@tabler/icons-react";

export const Card = ({
  label,
  children,
  contentPt,
  footer,
  dropdownMenu,
  sections,
}: {
  label: string;
  children?: React.ReactNode;
  contentPt?: number | string;
  footer?: React.ReactNode;
  sections?: React.ReactNode[];
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
      <div style={{ display: "flex", alignItems: "center" }}>
        <div style={{ flex: 1 }}>
          <Text
            style={{
              lineHeight: 1,
            }}
            c="dimmed"
            size="sm"
            fw="bold"
          >
            {label}
          </Text>
        </div>
        {dropdownMenu && (
          <Menu shadow="md" position="bottom-start">
            <Menu.Target>
              <ActionIcon size="sm">
                <IconMenu2 />
              </ActionIcon>
            </Menu.Target>

            <Menu.Dropdown>{dropdownMenu}</Menu.Dropdown>
          </Menu>
        )}
      </div>
    </MantineCard.Section>
    {sections &&
      sections.map((section, index) => (
        <MantineCard.Section key={index} withBorder inheritPadding py="xs">
          {section}
        </MantineCard.Section>
      ))}
    <Box pt={contentPt}>{children}</Box>
    {footer && (
      <MantineCard.Section withBorder inheritPadding py="xs">
        {footer}
      </MantineCard.Section>
    )}
  </MantineCard>
);
