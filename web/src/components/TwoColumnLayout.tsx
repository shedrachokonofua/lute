import { Grid } from "@mantine/core";

export const TwoColumnLayout = ({
  left,
  right,
  rightColumnRef,
}: {
  left: React.ReactNode;
  right: React.ReactNode;
  rightColumnRef?: React.RefObject<HTMLDivElement>;
}) => {
  return (
    <Grid m={0}>
      <Grid.Col
        md={2.75}
        style={{
          borderRight: "1px solid #DDD",
        }}
        sx={{
          "@media (min-width: 1024px)": {
            overflowY: "auto",
            height: "calc(100vh - 55px)",
          },
        }}
        px="md"
      >
        {left}
      </Grid.Col>
      <Grid.Col
        md={9.25}
        sx={{
          "@media (min-width: 1024px)": {
            overflowY: "auto",
            height: "calc(100vh - 55px)",
          },
          background: "#eee",
        }}
        px="xs"
        ref={rightColumnRef}
      >
        {right}
      </Grid.Col>
    </Grid>
  );
};
