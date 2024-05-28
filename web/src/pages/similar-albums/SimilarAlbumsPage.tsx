import { Button, Stack, Text, Title } from "@mantine/core";
import { QueryClient, useQuery } from "@tanstack/react-query";
import { Suspense, useEffect, useRef } from "react";
import {
  Await,
  Form,
  LoaderFunction,
  LoaderFunctionArgs,
  defer,
  useLoaderData,
  useRouteError,
} from "react-router-dom";
import { findSimilarAlbums, getAlbum } from "../../client";
import {
  AlbumCard,
  AlbumSearchFilters,
  CollapsibleSection,
  EmbeddingSimilaritySettings,
  TwoColumnLayout,
} from "../../components";
import { Page } from "../../components/Page";
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
  currentAlbum?: Album;
  settings?: SimilarAlbumsForm;
  similarAlbums?: Album[];
}

const getAlbumQuery = (fileName: string | undefined) => ({
  queryKey: ["album", fileName],
  queryFn: async () => (fileName ? await getAlbum(fileName) : undefined),
});

export const similarAlbumsPageLoader =
  (queryClient: QueryClient): LoaderFunction =>
  async ({ request }: LoaderFunctionArgs) => {
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
        : undefined;
    const currentAlbum = fileName
      ? await queryClient.ensureQueryData(getAlbumQuery(fileName))
      : undefined;
    const similarAlbums = settings ? findSimilarAlbums(settings) : undefined;

    return defer({
      currentAlbum,
      settings,
      similarAlbums,
    });
  };

export const Component = () => {
  const {
    currentAlbum: initialAlbum,
    settings,
    similarAlbums,
  } = useLoaderData() as SimilarAlbumsPageLoaderData;
  const updateSearchParams = useUpdateSearchParams();
  const rightColumnRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (rightColumnRef.current) {
      rightColumnRef.current.scrollTop = 0;
    }
  }, [settings?.fileName]);
  const { data: currentAlbum } = useQuery({
    ...getAlbumQuery(settings?.fileName),
    initialData: initialAlbum,
    enabled: !!settings?.fileName,
  });

  return (
    <Page>
      <TwoColumnLayout
        left={
          <Stack gap="lg">
            <Title order={4}>Settings</Title>
            <Form role="search">
              <Stack gap="md">
                <AlbumSearchInput initialAlbum={currentAlbum} />
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
              {(similarAlbums?: Album[]) => (
                <Stack gap="md">
                  {!similarAlbums ? (
                    <Text>Select an album to get started</Text>
                  ) : (
                    similarAlbums.map((album) => (
                      <AlbumCard
                        album={album}
                        actions={
                          <Button
                            onClick={() =>
                              updateSearchParams({
                                [FormName.FileName]: album.getFileName(),
                              })
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
    </Page>
  );
};
