import { VStack } from "./components";
import { Async } from "react-async";
import { findSimilarAlbums } from "./core";
import { AppContextValue } from "./types";
import { Album } from "./proto/lute_pb";

const loadSimilarAlbums = async ({ fileName }: { fileName: string }) => {
  if (!fileName) {
    return null;
  }
  return await findSimilarAlbums(fileName);
};

export const SimilarAlbums = ({ context }: { context: AppContextValue }) => {
  return (
    <VStack gap="0.25rem">
      <div>
        <div>Similar Albums</div>
      </div>
      <div>
        <Async
          promiseFn={loadSimilarAlbums as any}
          fileName={context.page?.fileName}
        >
          <Async.Rejected>
            {(error) => <div>Error: {error.message}</div>}
          </Async.Rejected>
          <Async.Fulfilled>
            {(albums: Album[]) => (
              <ul
                style={{
                  padding: 0,
                  margin: 0,
                }}
              >
                {albums.map((album) => (
                  <li key={album.getFileName()}>
                    <b>
                      {album
                        .getArtistsList()
                        .map((a) => a.getName())
                        .join(", ")}
                    </b>
                    {": "}
                    {album.getName()}
                  </li>
                ))}
              </ul>
            )}
          </Async.Fulfilled>
        </Async>
      </div>
    </VStack>
  );
};
