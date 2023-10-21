import { Button, Container, Stack, TextInput, Title } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconX } from "@tabler/icons-react";
import { useEffect } from "react";
import {
  ActionFunction,
  Form,
  redirect,
  useActionData,
} from "react-router-dom";
import { createProfile } from "../../client";

export const newProfileAction: ActionFunction = async ({ request }) => {
  try {
    const form = await request.formData();
    const id = form.get("id");
    const name = form.get("name");
    if (!id || !name) {
      throw new Error("Missing required fields");
    }
    await createProfile(id.toString(), name.toString());
    return redirect(`/profiles/${id}`);
  } catch (e) {
    return {
      error: (e as any).message,
    };
  }
};

export const NewProfilePage = () => {
  const actionData = useActionData() as {
    error: string;
  } | null;
  useEffect(() => {
    if (actionData?.error) {
      notifications.show({
        title: "Failed to create profile",
        message: actionData.error,
        color: "red",
        withBorder: true,
        icon: <IconX />,
      });
    }
  }, [actionData]);

  return (
    <Container py="xl">
      <Form method="post">
        <Stack
          spacing="md"
          style={{
            maxWidth: 400,
            margin: "auto",
          }}
        >
          <Title order={1} size="h3">
            Create a new profile
          </Title>
          <TextInput
            placeholder="Profile ID"
            label="Profile ID"
            description="The unique handle of the profile. 2-19 characters, letters, numbers, underscores, and dashes only"
            minLength={2}
            maxLength={19}
            required
            name="id"
          />
          <TextInput
            placeholder="Profile name"
            label="Profile name"
            description="The title of the profile. 2-40 characters"
            minLength={2}
            maxLength={40}
            required
            name="name"
          />
          <div>
            <Button type="submit">Submit</Button>
          </div>
        </Stack>
      </Form>
    </Container>
  );
};
