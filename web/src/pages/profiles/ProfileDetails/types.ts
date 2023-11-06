import { Album } from "../../../proto/lute_pb";

export type ProfileAlbumsList = {
  albums: Album[];
  searchMode: "new" | "existing";
  search: string;
  page: number;
  pageSize: number;
  pageCount: number;
  total: number;
};
