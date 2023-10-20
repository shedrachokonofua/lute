import { MantineProvider } from "@mantine/core";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { Layout } from "./components";
import {
  ProfilesPage,
  RecommendationPage,
  recommendationPageLoader,
  SpotifyOAuthCallbackPage,
} from "./pages";
import { DashboardPage } from "./pages/DashboardPage/DashboardPage";
import { getRemoteContext } from "./remote-context";

const router = createBrowserRouter([
  {
    path: "/spotify/oauth/callback",
    element: <SpotifyOAuthCallbackPage />,
  },
  {
    path: "/",
    element: <Layout />,
    id: "root",
    loader: getRemoteContext,
    shouldRevalidate: () => false,
    children: [
      {
        path: "*",
        element: <div>404</div>,
      },
      {
        path: "/",
        index: true,
        element: <DashboardPage />,
      },
      {
        path: "/profiles",
        element: <ProfilesPage />,
      },
      {
        path: "/recommendations",
        element: <RecommendationPage />,
        loader: recommendationPageLoader,
      },
    ],
  },
]);

export const App = () => (
  <MantineProvider>
    <RouterProvider router={router} />
  </MantineProvider>
);
