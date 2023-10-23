import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import { Layout } from "./components";
import {
  DashboardPage,
  NoProfileSelected,
  ProfileDetails,
  ProfilesPage,
  RecommendationPage,
  SpotifyOAuthCallbackPage,
  profileDetailsAction,
  profileDetailsLoader,
  profilePageAction,
  recommendationPageLoader,
} from "./pages";
import {
  NewProfilePage,
  newProfileAction,
} from "./pages/profiles/NewProfilePage";
import { getRemoteContext } from "./remote-context";

const queryClient = new QueryClient();

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
            action: profileDetailsAction(queryClient),
            loader: profileDetailsLoader(queryClient),
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
  <QueryClientProvider client={queryClient}>
    <MantineProvider>
      <Notifications autoClose={5000} />
      <RouterProvider router={router} />
    </MantineProvider>
  </QueryClientProvider>
);
