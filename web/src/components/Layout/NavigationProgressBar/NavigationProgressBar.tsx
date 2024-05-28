import { Box } from "@mantine/core";
import { useNavigation } from "react-router-dom";
import classes from "./NavigationProgressBar.module.css";

export const NavigationProgressBar = () => {
  const navigation = useNavigation();
  if (navigation.state === "idle") return null;

  return <Box className={classes.progressBar} />;
};
