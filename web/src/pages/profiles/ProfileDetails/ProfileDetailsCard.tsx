import { Card, Text } from "@mantine/core";

export const ProfileDetailsCard = ({
  label,
  children,
  footer,
}: {
  label: string;
  children?: React.ReactNode;
  footer?: React.ReactNode;
}) => (
  <Card withBorder shadow="xs">
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
    {children}
    {footer && (
      <Card.Section withBorder inheritPadding py="xs">
        {footer}
      </Card.Section>
    )}
  </Card>
);
