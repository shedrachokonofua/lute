import { Grid } from "@mantine/core";
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
  getProfile,
  getProfileSummary,
  putAlbumOnProfile,
  removeAlbumFromProfile,
  searchAlbums,
} from "../../../client";
import { Album, Profile, ProfileSummary } from "../../../proto/lute_pb";
import { ProfileAlbums } from "./ProfileAlbums";
import { ProfileDetailsCard } from "./ProfileDetailsCard";

interface ProfileDetailsLoaderData {
  profile: Profile;
  profileSummary: ProfileSummary;
  albumsList: {
    albums: Album[];
    search: string;
    page: number;
    pageCount: number;
  };
}

interface ProfileDetailsActionData {
  intent: string;
  ok: boolean;
  result?: any;
  error?: string;
}

const profileDetailsQuery = (id: string) => ({
  queryKey: ["profile", id],
  queryFn: async () => {
    const [profile, profileSummary] = await Promise.all([
      getProfile(id),
      getProfileSummary(id),
    ]);
    if (!profile || !profileSummary) {
      throw new Error("Profile not found");
    }
    return {
      profile,
      profileSummary,
    };
  },
});

export const profileDetailsAction =
  (queryClient: QueryClient): ActionFunction =>
  async ({ request, params }) => {
    const formData = await request.formData();
    const intent = formData.get("intent");

    if (intent === "delete-profile") {
      await deleteProfile(params.id as string);
      await queryClient.refetchQueries();
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
    const { profile, profileSummary } = await queryClient.ensureQueryData(
      profileDetailsQuery(id),
    );
    console.log("profile", profile.toObject());
    const searchParams = new URLSearchParams(new URL(request.url).search);
    const page = Number(searchParams.get("page")) || 1;
    const pageSize = Number(searchParams.get("pageSize")) || 5;
    const search = searchParams.get("search") || "";
    const searchResults = await searchAlbums(
      {
        text: search,
        includeFileNames: Array.from(profile.getAlbumsMap().keys()),
      },
      {
        offset: (page - 1) * pageSize,
        limit: pageSize,
      },
    );
    const albums = searchResults.getAlbumsList();
    const pageCount = Math.ceil(searchResults.getTotal() / pageSize);

    return {
      profile,
      profileSummary,
      albumsList: {
        albums,
        search,
        page,
        pageCount,
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
    albumsList,
  } = useLoaderData() as ProfileDetailsLoaderData;
  const {
    data: { profile },
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

  return (
    <Grid>
      <Grid.Col md={4}>
        <ProfileDetailsCard label="Summary"></ProfileDetailsCard>
      </Grid.Col>
      <Grid.Col md={8}>
        <ProfileAlbums profile={profile} list={albumsList} />
      </Grid.Col>
    </Grid>
  );
};
