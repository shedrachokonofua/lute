import React from "react";
import { MantineProvider } from "@mantine/core";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import {
  RecommendationPage,
  SpotifyOAuthCallbackPage,
  recommendationPageLoader,
} from "./pages";
import { Layout } from "./components";

const router = createBrowserRouter([
  {
    path: "/",
    element: <Layout />,
    children: [
      {
        index: true,
        element: <RecommendationPage />,
        loader: recommendationPageLoader,
      },
    ],
  },
  {
    path: "/spotify/oauth/callback",
    element: <SpotifyOAuthCallbackPage />,
  },
]);

export const App = () => (
  <MantineProvider children={undefined}>
    <RouterProvider router={router} />
  </MantineProvider>
);
