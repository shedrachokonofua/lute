import {
  Flex,
  Pagination,
  Select,
  Switch,
  Text,
  TextInput,
} from "@mantine/core";
import { useState } from "react";
import { Link } from "react-router-dom";
import { Card } from "../../../components";
import { useDebounce } from "../../../hooks/use-debounce";
import { useUpdateEffect } from "../../../hooks/use-update-effect";
import {
  getUpdatedQueryString,
  useUpdateSearchParams,
} from "../../../hooks/use-update-search-params";
import { Profile } from "../../../proto/lute_pb";
import { ProfileAlbumsListItem } from "./ProfileAlbumsListItem";
import { ProfileAlbumsList } from "./types";

interface ProfileAlbumsProps {
  profile: Profile;
  list: ProfileAlbumsList;
}

const AlbumSearchInput = ({ value }: { value: string }) => {
  const [searchValue, setSearchValue] = useState(value);
  const debouncedSearchValue = useDebounce(searchValue, 250);
  const updateSearchParams = useUpdateSearchParams();

  useUpdateEffect(() => {
    updateSearchParams({
      search: debouncedSearchValue,
      page: value !== debouncedSearchValue ? 1 : undefined,
    });
  }, [debouncedSearchValue]);

  useUpdateEffect(() => {
    if (!value) {
      setSearchValue("");
    }
  }, [value]);

  return (
    <TextInput
      placeholder="Search"
      variant="filled"
      value={searchValue}
      onChange={(e) => {
        setSearchValue(e.currentTarget.value);
      }}
    />
  );
};

const PageSizeSelect = ({ list }: { list: ProfileAlbumsList }) => {
  const updateSearchParams = useUpdateSearchParams();
  return (
    <Text>
      Showing{" "}
      <Select
        data={[
          { value: "5", label: "5" },
          { value: "10", label: "10" },
          { value: "25", label: "25" },
          { value: "50", label: "50" },
        ]}
        defaultValue={list.pageSize.toString()}
        styles={{
          root: {
            width: 70,
            display: "inline-block",
          },
          section: {
            paddingLeft: 4,
          },
        }}
        onChange={(pageSize) => {
          updateSearchParams({
            pageSize,
            page: 1,
          });
        }}
      />{" "}
      of {list.total} albums
    </Text>
  );
};

const getControlHref = (control: string, list: ProfileAlbumsProps["list"]) => {
  switch (control) {
    case "first":
      return getUpdatedQueryString({ page: 1 });
    case "last":
      return getUpdatedQueryString({ page: list.pageCount });
    case "next":
      return list.page < list.pageCount
        ? getUpdatedQueryString({ page: list.page + 1 })
        : undefined;
    case "previous":
      return list.page > 1
        ? getUpdatedQueryString({ page: list.page - 1 })
        : undefined;
    default:
      return undefined;
  }
};

const SearchModeSwitch = ({ list }: { list: ProfileAlbumsList }) => {
  const updateSearchParams = useUpdateSearchParams();
  return (
    <Switch
      onLabel={<Text>New</Text>}
      offLabel={<Text>Existing</Text>}
      size="lg"
      radius="lg"
      styles={{
        track: {
          background: "#DBDBDB",
          fontWeight: "normal",
          fontSize: 14,
          width: 90,
          textAlign: "center",
          minHeight: "2.25rem",
        },
      }}
      checked={list.searchMode === "new"}
      onChange={(e) => {
        updateSearchParams({
          searchMode: e.currentTarget.checked ? "new" : "existing",
          page: 1,
        });
      }}
    />
  );
};

export const ProfileAlbums = ({ profile, list }: ProfileAlbumsProps) => {
  return (
    <Card
      label="Albums"
      footer={
        <Flex justify="space-between" align="center">
          <Pagination
            value={list.page}
            total={list.pageCount}
            getItemProps={(page) => ({
              component: Link,
              to: getUpdatedQueryString({ page }),
            })}
            getControlProps={(control) => {
              const to = getControlHref(control, list);
              return to ? { component: Link, to } : {};
            }}
          />
          <PageSizeSelect list={list} />
        </Flex>
      }
      sections={[
        <Flex gap="md" align="center">
          <div
            style={{
              flex: 1,
            }}
          >
            <AlbumSearchInput value={list.search} />
          </div>
          <SearchModeSwitch list={list} />
        </Flex>,
      ]}
    >
      {list.albums.map((album, i) => (
        <ProfileAlbumsListItem
          key={album.getFileName()}
          album={album}
          profile={profile}
          searchMode={list.searchMode}
          factor={profile.getAlbumsMap().get(album.getFileName()) || 0}
          hasBorder={i !== list.albums.length - 1}
        />
      ))}
    </Card>
  );
};
