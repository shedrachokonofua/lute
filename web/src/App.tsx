import { MantineProvider, MantineThemeOverride } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import { Layout } from "./components";
import {
  DashboardPage,
  NewProfilePage,
  NoProfileSelected,
  ProfileDetails,
  ProfilesPage,
  RecommendationPage,
  SpotifyOAuthCallbackPage,
  newProfileAction,
  profileDetailsAction,
  profileDetailsLoader,
  profilePageAction,
  recommendationPageLoader,
} from "./pages";
import { getRemoteContext } from "./remote-context";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 10,
    },
  },
});

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

const theme: MantineThemeOverride = {
  fontFamily: "Ubuntu, sans-serif",
  colors: {
    blue: [
      "#E9F0FB",
      "#C2D6F5",
      "#9BBCEE",
      "#74A2E7",
      "#4C87E1",
      "#256DDA",
      "#1E57AE",
      "#164183",
      "#0F2C57",
      "#07162C",
    ],
    gray: [
      "#F2F2F2",
      "#DBDBDB",
      "#C4C4C4",
      "#ADADAD",
      "#969696",
      "#808080",
      "#666666",
      "#4D4D4D",
      "#333333",
      "#1A1A1A",
    ],
    red: [
      "#FAEAED",
      "#F2C5CC",
      "#E9A0AB",
      "#E07A8A",
      "#D85569",
      "#CF3049",
      "#A6263A",
      "#7C1D2C",
      "#53131D",
      "#290A0F",
    ],
  },
};

export const App = () => (
  <QueryClientProvider client={queryClient}>
    <MantineProvider theme={theme}>
      <Notifications autoClose={5000} />
      <RouterProvider router={router} />
    </MantineProvider>
  </QueryClientProvider>
);
