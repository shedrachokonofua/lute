import { MantineProvider, createTheme } from "@mantine/core";
import "@mantine/core/styles.css";
import { Notifications } from "@mantine/notifications";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import { Layout } from "./components";
import { newProfileAction } from "./pages/profiles/NewProfilePage";
import {
  profileDetailsAction,
  profileDetailsLoader,
} from "./pages/profiles/ProfileDetails/ProfileDetails";
import { profilePageAction } from "./pages/profiles/ProfilesPage";
import {
  createPlaylistAction,
  playlistPreviewPageLoader,
} from "./pages/recommendations/PlaylistPreviewPage";
import { recommendationPageLoader } from "./pages/recommendations/RecommendationPage";
import { similarAlbumsPageLoader } from "./pages/similar-albums";
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
    lazy: () => import("./pages/SpotifyOAuthCallbackPage"),
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
        lazy: () => import("./pages/dashboard/DashboardPage"),
      },
      {
        path: "/profiles/new",
        action: newProfileAction,
        lazy: () => import("./pages/profiles/NewProfilePage"),
      },
      {
        id: "profiles",
        path: "/profiles",
        lazy: () => import("./pages/profiles/ProfilesPage"),
        action: profilePageAction,
        children: [
          {
            index: true,
            lazy: () => import("./pages/profiles/NoProfileSelected"),
          },
          {
            id: "profile-details",
            path: "/profiles/:id",
            action: profileDetailsAction(queryClient),
            loader: profileDetailsLoader(queryClient),
            lazy: () =>
              import("./pages/profiles/ProfileDetails/ProfileDetails"),
          },
        ],
      },
      {
        path: "/recommendations",
        lazy: () => import("./pages/recommendations/RecommendationPage"),
        loader: recommendationPageLoader,
      },
      {
        path: "/recommendations/playlist",
        lazy: () => import("./pages/recommendations/PlaylistPreviewPage"),
        action: createPlaylistAction,
        loader: playlistPreviewPageLoader,
      },
      {
        path: "/similar-albums",
        lazy: () => import("./pages/similar-albums/SimilarAlbumsPage"),
        loader: similarAlbumsPageLoader(queryClient),
      },
    ],
  },
]);

const theme = createTheme({
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
});

export const App = () => (
  <QueryClientProvider client={queryClient}>
    <MantineProvider theme={theme}>
      <Notifications autoClose={5000} />
      <RouterProvider router={router} />
    </MantineProvider>
  </QueryClientProvider>
);
