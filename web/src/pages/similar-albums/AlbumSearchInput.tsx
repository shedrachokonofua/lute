import { Group, Select, SelectProps, Text } from "@mantine/core";
import { useState } from "react";
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

const searchResultItemFromAlbum = (album: Album): AlbumSearchResultItem => ({
  album,
  label: album.getName(),
  value: album.getFileName(),
});

const loadOptions: PromiseFn<AlbumSearchResultItem[] | undefined> = async ({
  query,
}) => {
  if (!query) return undefined;
  const searchResults = await searchAlbums(
    {
      text: query,
    },
    {
      offset: 0,
      limit: 10,
    },
  );
  return searchResults?.getAlbumsList().map(searchResultItemFromAlbum) || [];
};

const renderSelectOption: SelectProps["renderOption"] = ({
  option,
  checked,
}) => {
  let album = (option as any).album as Album;
  return (
    <Group wrap="nowrap">
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
  );
};

export const AlbumSearchInput = ({
  initialAlbum,
}: {
  initialAlbum?: Album;
}) => {
  const [searchValue, setSearchValue] = useState("");
  const [value, setValue] = useState(initialAlbum?.getFileName() || null);
  const debouncedSearchValue = useDebounce(searchValue, 250);
  const initialOptions = initialAlbum
    ? [searchResultItemFromAlbum(initialAlbum)]
    : undefined;
  const { data: options } = useAsync({
    promiseFn: loadOptions,
    query: debouncedSearchValue,
    watch: debouncedSearchValue,
  });
  console.log(options);

  return (
    <Select
      searchable
      name={FormName.FileName}
      onSearchChange={setSearchValue}
      searchValue={searchValue}
      label="Album"
      placeholder="Start typing to see options"
      data={options || initialOptions || []}
      value={value}
      onChange={setValue}
      renderOption={renderSelectOption}
      maxDropdownHeight={350}
      filter={({ options }) => options}
      required
    />
  );
};
