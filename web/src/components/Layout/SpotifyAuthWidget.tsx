import { Badge, Button } from "@mantine/core";
import { useAsync } from "react-async";
import { getSpotifyAuthUrl } from "../../client";
import { useRemoteContext } from "../../remote-context";

const redirect = async () => {
  try {
    window.location.assign(await getSpotifyAuthUrl());
  } catch (error) {
    alert("Something went wrong");
    console.error(error);
  }
};

export const SpotifyAuthWidget = () => {
  const { isSpotifyAuthenticated } = useRemoteContext();
  const { isLoading: isRedirectLoading, run: redirectForAuth } = useAsync({
    deferFn: redirect,
    onReject: (error) => {
      console.error(error);
    },
  });

  return isSpotifyAuthenticated ? (
    <Badge variant="gradient" gradient={{ to: "teal", from: "lime", deg: 105 }}>
      Spotify Connected
    </Badge>
  ) : (
    <Button
      size="compact-sm"
      variant="gradient"
      gradient={{ from: "teal", to: "lime", deg: 105 }}
      loading={isRedirectLoading}
      onClick={redirectForAuth}
    >
      Authenticate Spotify
    </Button>
  );
};
