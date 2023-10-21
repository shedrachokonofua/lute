import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { Layout } from "./components";
import {
  DashboardPage,
  NoProfileSelected,
  ProfileDetails,
  profileDetailsAction,
  profileDetailsLoader,
  profilePageAction,
  ProfilesPage,
  RecommendationPage,
  recommendationPageLoader,
  SpotifyOAuthCallbackPage,
} from "./pages";
import {
  newProfileAction,
  NewProfilePage,
} from "./pages/profiles/NewProfilePage";
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
    shouldRevalidate: ({ formData }) =>
      formData?.get("revalidate-remote-context") === "true",
    children: [
      {
        path: "*",
        element: <div>404</div>,
      },
      {
        index: true,
        element: <DashboardPage />,
      },
      {
        path: "/profiles/new",
        action: newProfileAction,
        element: <NewProfilePage />,
      },
      {
        id: "profiles",
        path: "/profiles",
        element: <ProfilesPage />,
        action: profilePageAction,
        children: [
          {
            index: true,
            element: <NoProfileSelected />,
          },
          {
            id: "profile-details",
            path: "/profiles/:id",
            action: profileDetailsAction,
            loader: profileDetailsLoader,
            element: <ProfileDetails />,
          },
        ],
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
    <Notifications autoClose={5000} />
    <RouterProvider router={router} />
  </MantineProvider>
);
