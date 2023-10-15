import { PageType } from "./proto/lute_pb";

export const getFileName = (urlString: string) => {
  const url = new URL(urlString);
  const base = decodeURI(
    url.pathname
      .split("/")
      .filter((x) => x !== "")
      .join("/")
  );
  const queryPart = base.startsWith("search") ? url.search : "";
  return base + queryPart;
};

export const pageTypeToString = (pageType: PageType) => {
  switch (pageType) {
    case PageType.ALBUMPAGE:
      return "album";
    case PageType.ARTISTPAGE:
      return "artist";
    case PageType.ALBUMSEARCHRESULTPAGE:
      return "album-search-result";
    case PageType.CHARTPAGE:
      return "chart";
  }
};
