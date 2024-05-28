import { Group, Menu, SegmentedControl, Stack, Text } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import {
  IconDownload,
  IconFolderHeart,
  IconPlaylist,
  IconTrashFilled,
  IconX,
} from "@tabler/icons-react";
import { useMutation, useQueries, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import {
  clearPendingSpotifyImports,
  importSavedSpotifyTracks,
  importSpotifyPlaylistTracks,
} from "../../../client";
import { Card } from "../../../components";
import { useInterval } from "../../../hooks/use-interval";
import {
  AggregatedStatus,
  GetPendingSpotifyImportsReply,
  Profile,
} from "../../../proto/lute_pb";
import { pendingSpotifyImportsQuery, profileDetailsQuery } from "./queries";

type CardMode = "in-progress" | "failures";

const usePendingSpotifyImports = (
  profile: Profile,
  initialPendingSpotifyImports: GetPendingSpotifyImportsReply,
  mode: CardMode,
) => {
  const profileId = profile.getId();
  const [
    {
      data: pendingSpotifyImports,
      refetch: refetchPendingSpotifyImports,
      isRefetching,
    },
    { refetch: refetchProfileDetails },
  ] = useQueries({
    queries: [
      {
        ...pendingSpotifyImportsQuery(profileId),
        initialData: initialPendingSpotifyImports,
      },
      {
        ...profileDetailsQuery(profileId),
        enabled: false,
      },
    ],
  });

  const [pendingImports, failedImports] = pendingSpotifyImports!
    .getStatusesList()
    .sort((a, b) => a.getStatus().localeCompare(b.getStatus()))
    .reduce<[AggregatedStatus[], AggregatedStatus[]]>(
      (acc, status) => {
        if (status.getStatus().includes("failed")) {
          acc[1].push(status);
        } else {
          acc[0].push(status);
        }
        return acc;
      },
      [[], []],
    );
  const isImportInProgress = pendingImports.length > 0;
  const secondsTillRefetch = useInterval(
    () => {
      refetchPendingSpotifyImports();
      refetchProfileDetails();
    },
    isImportInProgress && !isRefetching ? 5 : null,
  );

  const statusList = mode === "in-progress" ? pendingImports : failedImports;

  return {
    pendingSpotifyImports,
    statusList,
    isRefetching,
    isImportInProgress,
    secondsTillRefetch,
  };
};

const useImportMenu = (profile: Profile) => {
  const { refetch: refetchPendingSpotifyImports } = useQuery({
    ...pendingSpotifyImportsQuery(profile.getId()),
    enabled: false,
  });
  const loadingNotificationId = `import-${profile.getId()}`;
  const importSavedTracksMutation = useMutation({
    mutationFn: () => importSavedSpotifyTracks(profile.getId()),
    onMutate: () => {
      notifications.show({
        id: loadingNotificationId,
        message: "Loading saved tracks from Spotify",
        color: "gray",
        withBorder: true,
        loading: true,
        autoClose: false,
      });
    },
    onSuccess: () => {
      notifications.hide(loadingNotificationId);
      notifications.show({
        message: "Crawling RYM for albums, this may take a while",
        color: "blue",
        withBorder: true,
        icon: <IconDownload />,
      });
      refetchPendingSpotifyImports();
    },
    onError: (e) => {
      notifications.hide(loadingNotificationId);
      notifications.show({
        title: "Failed to import saved tracks",
        message: e.message,
        color: "red",
        withBorder: true,
        icon: <IconX />,
      });
    },
  });
  const importPlaylistMutation = useMutation({
    mutationFn: (playlistId: string) =>
      importSpotifyPlaylistTracks(profile.getId(), playlistId),
    onMutate: () => {
      notifications.show({
        id: loadingNotificationId,
        message: "Loading playlist tracks from Spotify",
        color: "blue",
        withBorder: true,
        loading: true,
        autoClose: false,
      });
    },
    onSuccess: () => {
      notifications.hide(loadingNotificationId);
      notifications.show({
        message: "Looking up playlist tracks on RYM, this may take a while",
        color: "blue",
        withBorder: true,
        icon: <IconDownload />,
      });
      refetchPendingSpotifyImports();
    },
    onError: (e) => {
      notifications.hide(loadingNotificationId);
      notifications.show({
        title: "Failed to import playlist tracks",
        message: e.message,
        color: "red",
        withBorder: true,
        icon: <IconX />,
      });
    },
  });
  const clearPendingSpotifyImportsMutation = useMutation({
    mutationFn: () => clearPendingSpotifyImports(profile.getId()),
    onMutate: () => {
      notifications.show({
        id: loadingNotificationId,
        message: "Clearing pending imports",
        color: "gray",
        withBorder: true,
        loading: true,
        autoClose: false,
      });
    },
    onSuccess: () => {
      notifications.hide(loadingNotificationId);
      notifications.show({
        message: "Cleared pending imports",
        withBorder: true,
        icon: <IconTrashFilled />,
      });
      refetchPendingSpotifyImports();
    },
    onError: (e) => {
      notifications.hide(loadingNotificationId);
      notifications.show({
        title: "Failed to clear pending imports",
        message: e.message,
        color: "red",
        withBorder: true,
        icon: <IconX />,
      });
    },
  });

  return {
    importSavedTracksMutation,
    importPlaylistMutation,
    clearPendingSpotifyImportsMutation,
  };
};

const statusToName: Record<string, string> = {
  started: "Started",
  search_crawling: "Crawling Search Page",
  search_parsing: "Parsing Search Page",
  album_crawling: "Crawling Album Page",
  album_parsing: "Parsing Album Page",
  search_parsed: "Parsed Search Page",
  search_parse_failed: "Failed to Parse Search Page",
  album_parse_failed: "Failed to Parse Album Page",
};

export const ProfileSpotifyImport = ({
  profile,
  pendingSpotifyImports: initialPendingSpotifyImports,
}: {
  profile: Profile;
  pendingSpotifyImports: GetPendingSpotifyImportsReply;
}) => {
  const [type, setType] = useState<CardMode>("in-progress");
  const {
    pendingSpotifyImports,
    statusList,
    isImportInProgress,
    secondsTillRefetch,
    isRefetching,
  } = usePendingSpotifyImports(profile, initialPendingSpotifyImports, type);
  const {
    importSavedTracksMutation,
    importPlaylistMutation,
    clearPendingSpotifyImportsMutation,
  } = useImportMenu(profile);

  return (
    <Card
      label="Spotify Import"
      dropdownMenu={
        <>
          <Menu.Item
            disabled={importSavedTracksMutation.isPending || isImportInProgress}
            onClick={() => importSavedTracksMutation.mutate()}
            leftSection={<IconFolderHeart size={16} />}
          >
            Import from saved tracks
          </Menu.Item>
          <Menu.Item
            disabled={importPlaylistMutation.isPending || isImportInProgress}
            onClick={() => {
              const playlistId = window.prompt("Enter the Spotify playlist ID");
              if (playlistId) {
                importPlaylistMutation.mutate(playlistId);
              }
            }}
            leftSection={<IconPlaylist size={16} />}
          >
            Import from playlist
          </Menu.Item>
          <Menu.Item
            disabled={
              clearPendingSpotifyImportsMutation.isPending ||
              pendingSpotifyImports?.getCount() === 0
            }
            onClick={() => clearPendingSpotifyImportsMutation.mutate()}
            leftSection={<IconTrashFilled size={16} />}
          >
            Clear pending imports
          </Menu.Item>
        </>
      }
      footer={
        isImportInProgress && secondsTillRefetch ? (
          <Text size="xs" c="gray" ta="right">
            {isRefetching
              ? "Refetching..."
              : `Refetching in ${secondsTillRefetch} seconds`}
          </Text>
        ) : undefined
      }
    >
      <Stack gap="md" py="sm">
        <SegmentedControl
          data={[
            { label: "In-Progress", value: "in-progress" },
            { label: "Failures", value: "failures" },
          ]}
          value={type}
          onChange={(value) => setType(value as CardMode)}
        />
        <div>
          {statusList.map((aggStatus, i) => {
            const status = aggStatus.getStatus();
            const count = aggStatus.getCount();
            return (
              <Group
                key={status}
                justify="space-between"
                py={6}
                style={{
                  borderBottom:
                    i !== statusList.length - 1 ? "1px solid #ddd" : undefined,
                }}
              >
                <Text size="md">{statusToName[status] || status}</Text>
                <Text size="md">{count}</Text>
              </Group>
            );
          })}
        </div>
      </Stack>
    </Card>
  );
};
