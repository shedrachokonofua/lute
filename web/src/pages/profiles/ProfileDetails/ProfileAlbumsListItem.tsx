import { ActionIcon, Group, NumberInput, Text } from "@mantine/core";
import { IconTrash } from "@tabler/icons-react";
import { useState } from "react";
import { Form, useSubmit } from "react-router-dom";
import { Album, Profile } from "../../../proto/lute_pb";

const AlbumFactorInput = ({
  album,
  factor,
}: {
  album: Album;
  factor: number;
}) => {
  /**
   * The factor value state that controls the input is only updated when the form is submitted. This enables
   * automatically resetting the value when the user changes it and then blurs without submitting.
   */
  const [factorValue, setFactorValue] = useState(factor);

  return (
    <Form
      method="post"
      onSubmit={(e) => {
        if (
          !confirm(
            `Are you sure you want to the factor for "${album.getName()}"`,
          )
        ) {
          e.preventDefault();
        }
        const data = new FormData(e.target as HTMLFormElement);
        const factor = Number(data.get("factor"));
        if (factor < 1) {
          alert("Factor must be greater than 0");
          e.preventDefault();
          return;
        }
        setFactorValue(factor);
      }}
    >
      <NumberInput
        name="factor"
        variant="filled"
        value={factorValue}
        min={1}
        rightSectionWidth={75}
        rightSection={
          <Group spacing={4}>
            <Text size="xs"> of </Text>
            <Text>{album.getTracksList().length}</Text>
          </Group>
        }
        styles={{
          input: {
            fontSize: "1rem",
            textAlign: "right",
          },
          root: {
            width: 110,
          },
        }}
      />
      <input type="hidden" name="fileName" value={album.getFileName()} />
      <input type="hidden" name="intent" value="update-album-factor" />
    </Form>
  );
};

export const ProfileAlbumsListItem = ({
  profile,
  album,
  factor,
}: {
  profile: Profile;
  album: Album;
  factor: number;
}) => {
  const submit = useSubmit();

  return (
    <Group
      style={{
        borderBottom: "1px solid #DDD",
        padding: "8px 0",
      }}
    >
      <img
        src={album.getCoverImageUrl()}
        alt={album.getName()}
        style={{
          width: 75,
          minHeight: 75,
        }}
      />
      <div
        style={{
          flex: 1,
        }}
      >
        <Text>{album.getName()}</Text>
        <Text>
          {album
            .getArtistsList()
            .map((a) => a.getName())
            .join(", ")}
        </Text>
        <Text size="sm">{album.getPrimaryGenresList().join(", ")}</Text>
      </div>
      <Group spacing="xl">
        <div>
          <AlbumFactorInput album={album} factor={factor} />
        </div>
        <div>
          <ActionIcon
            color="red"
            variant="light"
            radius="sm"
            onClick={() => {
              if (
                confirm(
                  `Are you sure you want to remove "${album.getName()}" from "${profile.getName()}"`,
                )
              ) {
                submit(
                  {
                    intent: "remove-album",
                    fileName: album.getFileName(),
                  },
                  {
                    method: "delete",
                  },
                );
              }
            }}
          >
            <IconTrash size="1rem" />
          </ActionIcon>
        </div>
      </Group>
    </Group>
  );
};
