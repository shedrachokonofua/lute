import { Button, Stack, Text, Title } from "@mantine/core";
import { Suspense, useEffect, useRef } from "react";
import {
  Await,
  Form,
  LoaderFunctionArgs,
  defer,
  useLoaderData,
  useRouteError,
} from "react-router-dom";
import { findSimilarAlbums } from "../../client";
import {
  AlbumCard,
  AlbumSearchFilters,
  CollapsibleSection,
  EmbeddingSimilaritySettings,
  TwoColumnLayout,
} from "../../components";
import { FormName, parseAlbumSearchFiltersForm } from "../../forms";
import { useUpdateSearchParams } from "../../hooks/use-update-search-params";
import { Album } from "../../proto/lute_pb";
import { AlbumSearchInput } from "./AlbumSearchInput";
import { SimilarAlbumsForm } from "./types";

const ErrorBoundary = () => {
  const error = useRouteError();
  console.error(error);
  return <div>Dang! Something went wrong.</div>;
};

export interface SimilarAlbumsPageLoaderData {
  settings: SimilarAlbumsForm | null;
  similarAlbums: Album[] | null;
}

export const similarAlbumsPageLoader = async ({
  request,
}: LoaderFunctionArgs) => {
  const url = new URL(request.url);
  const fileName = url.searchParams.get(FormName.FileName);
  const embeddingKey = url.searchParams.get(
    FormName.EmbeddingSimilarityEmbeddingKey,
  );
  const settings =
    fileName && embeddingKey
      ? {
          fileName,
          embeddingKey,
          filters: parseAlbumSearchFiltersForm(url),
          limit: 50,
        }
      : null;
  const similarAlbums = settings ? findSimilarAlbums(settings) : null;

  return defer({
    settings,
    similarAlbums,
  });
};

export const SimilarAlbumsPage = () => {
  const { settings, similarAlbums } =
    useLoaderData() as SimilarAlbumsPageLoaderData;
  const updateSearchParams = useUpdateSearchParams();
  const rightColumnRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (rightColumnRef.current) {
      rightColumnRef.current.scrollTop = 0;
    }
  }, [settings?.fileName]);

  return (
    <TwoColumnLayout
      left={
        <Stack spacing="lg">
          <Title order={4}>Settings</Title>
          <Form role="search">
            <Stack spacing="md">
              <AlbumSearchInput />
              <EmbeddingSimilaritySettings />
              <CollapsibleSection title="Filters">
                <AlbumSearchFilters />
              </CollapsibleSection>
              <div>
                <Button type="submit" fullWidth>
                  Submit
                </Button>
              </div>
            </Stack>
          </Form>
        </Stack>
      }
      rightColumnRef={rightColumnRef}
      right={
        <Suspense fallback={<Text>Loading recommendations...</Text>}>
          <Await resolve={similarAlbums} errorElement={<ErrorBoundary />}>
            {(similarAlbums: Album[] | null) => (
              <Stack spacing="md">
                {similarAlbums === null ? (
                  <Text>Select an album to get started</Text>
                ) : (
                  similarAlbums.map((album) => (
                    <AlbumCard
                      album={album}
                      actions={
                        <Button
                          onClick={() =>
                            updateSearchParams(
                              {
                                [FormName.FileName]: album.getFileName(),
                              },
                              {
                                preventScrollReset: false,
                              },
                            )
                          }
                        >
                          View similar albums
                        </Button>
                      }
                    />
                  ))
                )}
              </Stack>
            )}
          </Await>
        </Suspense>
      }
    />
  );
};
