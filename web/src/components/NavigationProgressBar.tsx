import { Box } from "@mantine/core";
import { useNavigation } from "react-router-dom";

export const NavigationProgressBar = () => {
  const navigation = useNavigation();
  if (navigation.state === "idle") return null;

  return (
    <Box
      sx={{
        position: "fixed",
        top: 0,
        left: 0,
        height: 3,
        background: "#CCC",
        zIndex: 1000,
        animation: "loadingAnimation 2s forwards",

        "@keyframes loadingAnimation": {
          "0%": {
            width: 0,
          },
          "50%": {
            width: "50%",
          },
          "100%": {
            width: "95%",
          },
        },
      }}
    />
  );
};
