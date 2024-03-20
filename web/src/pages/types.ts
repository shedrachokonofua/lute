export interface AlbumSearchFiltersForm {
  count: number | undefined;
  minReleaseYear: number | undefined;
  maxReleaseYear: number | undefined;
  includePrimaryGenres: string[] | undefined;
  excludePrimaryGenres: string[] | undefined;
  includeSecondaryGenres: string[] | undefined;
  excludeSecondaryGenres: string[] | undefined;
  includeLanguages: string[] | undefined;
  excludeLanguages: string[] | undefined;
  excludeKnownArtists: number | undefined;
}
