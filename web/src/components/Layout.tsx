import { Button, Container, Title, Flex, Badge, Header } from "@mantine/core";
import React from "react";
import { Async, useAsync } from "react-async";
import { getSpotifyAuthUrl, isSpotifyAuthorized } from "../client";

const redirect = async () => {
  try {
    window.location.assign(await getSpotifyAuthUrl());
  } catch (error) {
    alert("Something went wrong");
    console.error(error);
  }
};

const SpotifyWidget = () => {
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
              variant="gradient"
              gradient={{ from: "indigo", to: "cyan" }}
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

export const Layout = ({ children }: React.FC) => (
  <main>
    <Header>
      <Container size="xl">
        <Flex mih={80} align="center" justify="space-between">
          <Title
            order={1}
            size="h1"
            weight="normal"
            sx={{
              fontFamily: "YoungSerif",
              letterSpacing: "-1.5px",
            }}
          >
            `lute
          </Title>
          <SpotifyWidget />
        </Flex>
      </Container>
    </Header>
    <div style={{ paddingTop: "2rem" }}>{children}</div>
  </main>
);
