import { Grid } from "@mantine/core";
import classes from "./TwoColumnLayout.module.css";

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
    <Grid
      gutter={0}
      styles={{
        root: {
          flexGrow: 1,
          display: "flex",
          overflowY: "hidden",
        },
        inner: {
          flexGrow: 1,
          height: "100%",
        },
      }}
    >
      <Grid.Col span={{ md: 2.75 }} className={classes.left} px="md" py="sm">
        {left}
      </Grid.Col>
      <Grid.Col
        span={{ md: 9.25 }}
        className={classes.right}
        px="0"
        py="0"
        ref={rightColumnRef}
      >
        {right}
      </Grid.Col>
    </Grid>
  );
};
