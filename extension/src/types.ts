import { PageType, Profile } from "./proto/lute_pb";

export interface PageContextValue {
  pageType: PageType;
  fileName: string;
}

export interface AppContextValue {
  page: PageContextValue | undefined;
  profiles: Profile[];
}
