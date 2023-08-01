import React from "react";
import { Badge, Button } from "@mantine/core";
import { useAsync, Async } from "react-async";
import { getSpotifyAuthUrl, isSpotifyAuthorized } from "../client";

const redirect = async () => {
  try {
    window.location.assign(await getSpotifyAuthUrl());
  } catch (error) {
    alert("Something went wrong");
    console.error(error);
  }
};

export const SpotifyWidget = () => {
  const { isLoading: isRedirectLoading, run: redirectForAuth } = useAsync({
    deferFn: redirect,
    onReject: (error) => {
      console.error(error);
    },
  });

  return (
    <Async promiseFn={isSpotifyAuthorized}>
      <Async.Loading>Loading...</Async.Loading>
      <Async.Rejected>
        Failed to get spotify status. Refresh page.
      </Async.Rejected>
      <Async.Fulfilled>
        {(isAuthenticated: boolean) =>
          isAuthenticated ? (
            <Badge
              variant="gradient"
              gradient={{ from: "teal", to: "lime", deg: 105 }}
            >
              Spotify Connected
            </Badge>
          ) : (
            <Button
              compact
              variant="gradient"
              gradient={{ from: "teal", to: "lime", deg: 105 }}
              loading={isRedirectLoading}
              onClick={redirectForAuth}
            >
              Authenticate Spotify
            </Button>
          )
        }
      </Async.Fulfilled>
    </Async>
  );
};
