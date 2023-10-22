import { Group } from "@mantine/core";
import { Link } from "react-router-dom";
import { Album, Profile } from "../../../proto/lute_pb";
import { ProfileAlbumsListItem } from "./ProfileAlbumsListItem";
import { ProfileDetailsCard } from "./ProfileDetailsCard";

interface ProfileAlbumsProps {
  profile: Profile;
  list: {
    albums: Album[];
    page: number;
    pageCount: number;
  };
}

const getLinkWithUpdatedPage = (page: number) => {
  const url = new URL(window.location.href);
  const searchParams = new URLSearchParams(url.search);
  searchParams.set("albumPage", String(page));
  return "?" + searchParams.toString();
};

const Pagination = ({ list }: { list: ProfileAlbumsProps["list"] }) => {
  const hasNext = list.page < list.pageCount;
  const hasPrevious = list.page > 1;

  return (
    <Group>
      {hasPrevious && (
        <Link to={getLinkWithUpdatedPage(list.page - 1)}>Previous</Link>
      )}
      {hasNext && <Link to={getLinkWithUpdatedPage(list.page + 1)}>Next</Link>}
    </Group>
  );
};

export const ProfileAlbums = ({ profile, list }: ProfileAlbumsProps) => {
  return (
    <ProfileDetailsCard
      label={`Albums(${profile.getAlbumsMap().getLength()})`}
      footer={<Pagination list={list} />}
    >
      <div>
        {list.albums.map((album) => (
          <ProfileAlbumsListItem
            key={album.getFileName()}
            album={album}
            profile={profile}
            factor={profile.getAlbumsMap().get(album.getFileName()) || 0}
          />
        ))}
      </div>
    </ProfileDetailsCard>
  );
};
