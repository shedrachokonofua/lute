import { Card, Group, TextInput } from "@mantine/core";
import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { Album, Profile } from "../../../proto/lute_pb";
import { ProfileAlbumsListItem } from "./ProfileAlbumsListItem";
import { ProfileDetailsCard } from "./ProfileDetailsCard";
import { useDebounce } from "./use-debounce";

interface ProfileAlbumsProps {
  profile: Profile;
  list: {
    albums: Album[];
    search: string;
    page: number;
    pageCount: number;
  };
}

const getUpdatedQueryString = (updates: Record<string, any>) => {
  const url = new URL(window.location.href);
  const searchParams = new URLSearchParams(url.search);
  for (const [key, value] of Object.entries(updates)) {
    if (value !== undefined) {
      searchParams.set(key, value);
    }
  }
  return "?" + searchParams.toString();
};

const PaginationLink = ({
  targetPage,
  enabled,
  label,
}: {
  targetPage: number;
  enabled: boolean;
  label: string;
}) => {
  return enabled ? (
    <Link to={getUpdatedQueryString({ page: targetPage })}>{label}</Link>
  ) : (
    <div>{label}</div>
  );
};

const Pagination = ({ list }: { list: ProfileAlbumsProps["list"] }) => (
  <Group>
    <PaginationLink
      targetPage={list.page - 1}
      enabled={list.page > 1}
      label="Previous"
    />
    <PaginationLink
      targetPage={list.page + 1}
      enabled={list.page < list.pageCount}
      label="Next"
    />
  </Group>
);

const AlbumSearchInput = ({ value }: { value: string }) => {
  const [searchValue, setSearchValue] = useState(value);
  const debouncedSearchValue = useDebounce(searchValue, 250);
  const navigate = useNavigate();
  useEffect(() => {
    navigate(
      getUpdatedQueryString({
        search: debouncedSearchValue,
        page: value !== debouncedSearchValue ? 1 : undefined,
      }),
    );
  }, [debouncedSearchValue]);
  useEffect(() => {
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

export const ProfileAlbums = ({ profile, list }: ProfileAlbumsProps) => {
  return (
    <ProfileDetailsCard
      label={`Albums(${profile.getAlbumsMap().getLength()})`}
      footer={<Pagination list={list} />}
    >
      <Card.Section withBorder inheritPadding py="xs">
        <AlbumSearchInput value={list.search} />
      </Card.Section>
      <div>
        {list.albums.map((album, i) => (
          <ProfileAlbumsListItem
            key={album.getFileName()}
            album={album}
            profile={profile}
            factor={profile.getAlbumsMap().get(album.getFileName()) || 0}
            hasBorder={i !== list.albums.length - 1}
          />
        ))}
      </div>
    </ProfileDetailsCard>
  );
};
