import { Group, Select, Text } from "@mantine/core";
import { forwardRef, useState } from "react";
import { PromiseFn, useAsync } from "react-async";
import { searchAlbums } from "../../client";
import { FormName } from "../../forms";
import { useDebounce } from "../../hooks/use-debounce";
import { Album } from "../../proto/lute_pb";

interface AlbumSearchResultItem {
  album: Album;
  label: string;
  value: string;
}

const loadOptions: PromiseFn<AlbumSearchResultItem[]> = async ({ query }) => {
  if (!query) return [];

  const searchResults = await searchAlbums(
    {
      text: query,
    },
    {
      limit: 10,
    },
  );

  return (
    searchResults?.getAlbumsList().map((album) => ({
      label: album.getName(),
      value: album.getFileName(),
      album,
    })) || []
  );
};

type ItemProps = React.ComponentPropsWithoutRef<"div"> & AlbumSearchResultItem;

const SelectItem = forwardRef<HTMLDivElement, AlbumSearchResultItem>(
  ({ album, ...others }: ItemProps, ref) => (
    <div ref={ref} {...others}>
      <Group noWrap>
        <div>
          <img
            src={album.getCoverImageUrl()}
            style={{ width: 40, height: 40, borderRadius: 4 }}
          />
        </div>
        <div>
          <Text size="sm">{album.getName()}</Text>
          <Text size="xs" opacity={0.65}>
            {album
              .getArtistsList()
              .map((artist) => artist.getName())
              .join(", ")}
          </Text>
        </div>
      </Group>
    </div>
  ),
);

export const AlbumSearchInput = () => {
  const [searchValue, setSearchValue] = useState("");
  const debouncedSearchValue = useDebounce(searchValue, 250);
  const { data: options } = useAsync({
    promiseFn: loadOptions,
    query: debouncedSearchValue,
    watch: debouncedSearchValue,
  });

  return (
    <Select
      searchable
      name={FormName.FileName}
      onSearchChange={setSearchValue}
      searchValue={searchValue}
      label="Album"
      placeholder="Start typing to see options"
      data={options || []}
      itemComponent={SelectItem}
      maxDropdownHeight={350}
      required
    />
  );
};
