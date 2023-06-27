import React from "react";
import { MantineProvider } from "@mantine/core";
import { Route, Router, Switch } from "wouter";
import { IndexPage, SpotifyOAuthCallbackPage } from "./pages";
import { Layout } from "./components";

export const App = () => (
  <MantineProvider children={undefined}>
    <Switch>
      <Route path="/spotify/oauth/callback">
        <SpotifyOAuthCallbackPage />
      </Route>
      <Layout>
        <Route path="/">
          <IndexPage />
        </Route>
      </Layout>
    </Switch>
  </MantineProvider>
);
