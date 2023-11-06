import { Grid, Stack } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconX } from "@tabler/icons-react";
import { QueryClient, useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import {
  ActionFunction,
  LoaderFunction,
  redirect,
  useActionData,
  useLoaderData,
  useParams,
} from "react-router-dom";
import {
  deleteProfile,
  putAlbumOnProfile,
  removeAlbumFromProfile,
  searchAlbums,
} from "../../../client";
import {
  Album,
  GetPendingSpotifyImportsReply,
  Profile,
  ProfileSummary,
} from "../../../proto/lute_pb";
import { useRemoteContext } from "../../../remote-context";
import { ProfileAlbums } from "./ProfileAlbums";
import { ProfileOverview } from "./ProfileOverview";
import { ProfileSpotifyImport } from "./ProfileSpotifyImport";
import { pendingSpotifyImportsQuery, profileDetailsQuery } from "./queries";

interface ProfileDetailsLoaderData {
  profile: Profile;
  profileSummary: ProfileSummary;
  pendingSpotifyImports: GetPendingSpotifyImportsReply;
  albumsList: {
    albums: Album[];
    search: string;
    page: number;
    pageCount: number;
    pageSize: number;
    total: number;
  };
}

interface ProfileDetailsActionData {
  intent: string;
  ok: boolean;
  result?: any;
  error?: string;
}

export const profileDetailsAction =
  (queryClient: QueryClient): ActionFunction =>
  async ({ request, params }) => {
    const formData = await request.formData();
    const intent = formData.get("intent");

    if (intent === "delete-profile") {
      await deleteProfile(params.id as string);
      return redirect("/profiles");
    }
    if (intent === "remove-album") {
      try {
        await removeAlbumFromProfile(
          params.id as string,
          formData.get("fileName") as string,
        );
        await queryClient.refetchQueries();
        return {
          intent,
          ok: true,
        };
      } catch (e) {
        return {
          intent,
          ok: false,
          error: (e as any).message,
        };
      }
    }
    if (intent === "update-album-factor") {
      try {
        const fileName = formData.get("fileName") as string;
        const factor = Number(formData.get("factor"));
        await putAlbumOnProfile(params.id as string, fileName, factor);
        await queryClient.refetchQueries();
        return {
          intent,
          ok: true,
        };
      } catch (e) {
        return {
          intent,
          ok: false,
          error: (e as any).message,
        };
      }
    }

    return null;
  };

export const profileDetailsLoader =
  (queryClient: QueryClient): LoaderFunction =>
  async ({ params, request }) => {
    const id = params.id as string;
    const [{ profile, profileSummary }, pendingSpotifyImports] =
      await Promise.all([
        queryClient.ensureQueryData(profileDetailsQuery(id)),
        queryClient.ensureQueryData(pendingSpotifyImportsQuery(id)),
      ]);
    const searchParams = new URLSearchParams(new URL(request.url).search);
    const page = Number(searchParams.get("page")) || 1;
    const pageSize = Number(searchParams.get("pageSize")) || 5;
    const search = searchParams.get("search") || "";
    const fileNames = Array.from(profile.getAlbumsMap().keys());
    const searchResults =
      fileNames.length > 0
        ? await searchAlbums(
            {
              text: search.trim(),
              includeFileNames: Array.from(profile.getAlbumsMap().keys()),
            },
            {
              offset: (page - 1) * pageSize,
              limit: pageSize,
            },
          )
        : null;
    const albums = searchResults?.getAlbumsList() || [];
    const total = searchResults?.getTotal() || 0;
    const pageCount = Math.ceil(total / pageSize);

    return {
      profile,
      profileSummary,
      pendingSpotifyImports,
      albumsList: {
        albums,
        search,
        page,
        pageSize,
        pageCount,
        total,
      },
    } as ProfileDetailsLoaderData;
  };

const successNotification = (message: string) => ({
  message,
  color: "blue",
  withBorder: true,
  icon: <IconCheck />,
});

const errorNotification = (title: string, message: string) => ({
  title,
  message,
  color: "red",
  withBorder: true,
  icon: <IconX />,
});

const getActionNotification = (actionData: ProfileDetailsActionData) => {
  if (!actionData.intent) return null;
  if (actionData.intent === "remove-album") {
    return actionData.ok === true
      ? successNotification("Album removed from profile")
      : errorNotification(
          "Failed to remove album from profile",
          actionData.error as string,
        );
  }
  if (actionData.intent === "update-album-factor") {
    return actionData.ok === true
      ? successNotification("Album factor updated")
      : errorNotification(
          "Failed to update album factor",
          actionData.error as string,
        );
  }
  return null;
};

export const ProfileDetails = () => {
  const params = useParams();
  const {
    profile: initialProfile,
    profileSummary: initialProfileSummary,
    pendingSpotifyImports: pendingSpotifyImports,
    albumsList,
  } = useLoaderData() as ProfileDetailsLoaderData;
  const {
    data: { profile, profileSummary },
  } = useQuery({
    ...profileDetailsQuery(params.id as string),
    initialData: {
      profile: initialProfile,
      profileSummary: initialProfileSummary,
    },
  });
  const actionData = useActionData() as ProfileDetailsActionData | null;
  useEffect(() => {
    if (!actionData) return;
    const notification = getActionNotification(actionData);
    if (notification) {
      notifications.show(notification);
    }
  }, [actionData]);
  const { isSpotifyAuthenticated } = useRemoteContext();

  return (
    <Grid>
      <Grid.Col md={4}>
        <Stack spacing="md">
          <ProfileOverview profileSummary={profileSummary} />
          {isSpotifyAuthenticated && (
            <ProfileSpotifyImport
              profile={profile}
              pendingSpotifyImports={pendingSpotifyImports}
            />
          )}
        </Stack>
      </Grid.Col>
      <Grid.Col md={8}>
        <ProfileAlbums profile={profile} list={albumsList} />
      </Grid.Col>
    </Grid>
  );
};
