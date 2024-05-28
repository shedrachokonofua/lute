import { Checkbox, Grid, MultiSelect, NumberInput, Stack } from "@mantine/core";
import { AlbumSearchFiltersForm, FormName } from "../forms";
import { useRemoteContext } from "../remote-context";

export const AlbumSearchFilters = ({
  filters,
}: {
  filters?: AlbumSearchFiltersForm;
}) => {
  const { albumMonitor } = useRemoteContext();

  const primaryGenreOptions = albumMonitor
    .getAggregatedGenresList()
    .map((genre) => ({
      label: `${genre.getName()} (${genre.getPrimaryGenreCount()})`,
      value: genre.getName(),
    }));
  const secondaryGenreOptions = albumMonitor
    .getAggregatedGenresList()
    .map((genre) => ({
      label: `${genre.getName()} (${genre.getSecondaryGenreCount()})`,
      value: genre.getName(),
    }));
  const languageOptions = albumMonitor
    .getAggregatedLanguagesList()
    .map((language) => ({
      label: `${language.getName()} (${language.getCount()})`,
      value: language.getName(),
    }));

  return (
    <Stack gap="sm">
      <Grid gutter="xs">
        <Grid.Col span={6}>
          <NumberInput
            label="Min Release Year"
            placeholder="Year"
            min={1900}
            max={new Date().getFullYear()}
            step={1}
            name={FormName.MinReleaseYear}
            defaultValue={filters?.minReleaseYear}
            variant="filled"
          />
        </Grid.Col>
        <Grid.Col span={6}>
          <NumberInput
            label="Max Release Year"
            placeholder="Year"
            min={1900}
            max={2023}
            step={1}
            name={FormName.MaxReleaseYear}
            defaultValue={filters?.maxReleaseYear}
            variant="filled"
          />
        </Grid.Col>
      </Grid>
      <MultiSelect
        label="Include Primary Genres"
        data={primaryGenreOptions}
        placeholder="Select Genres"
        name={FormName.IncludePrimaryGenres}
        defaultValue={filters?.includePrimaryGenres}
        variant="filled"
        searchable
      />
      <MultiSelect
        label="Exclude Primary Genres"
        data={primaryGenreOptions}
        placeholder="Select Genres"
        name={FormName.ExcludePrimaryGenres}
        defaultValue={filters?.excludePrimaryGenres}
        variant="filled"
        searchable
      />
      <MultiSelect
        label="Include Secondary Genres"
        data={secondaryGenreOptions}
        placeholder="Select Genres"
        name={FormName.IncludeSecondaryGenres}
        defaultValue={filters?.includeSecondaryGenres}
        variant="filled"
        searchable
      />
      <MultiSelect
        label="Exclude Secondary Genres"
        data={secondaryGenreOptions}
        placeholder="Select Genres"
        name={FormName.ExcludeSecondaryGenres}
        defaultValue={filters?.excludeSecondaryGenres}
        variant="filled"
        searchable
      />
      <MultiSelect
        label="Include Languages"
        data={languageOptions}
        placeholder="Select Languages"
        name={FormName.IncludeLanguages}
        defaultValue={filters?.includeLanguages}
        variant="filled"
        searchable
      />
      <MultiSelect
        label="Exclude Languages"
        data={languageOptions}
        placeholder="Select Languages"
        name={FormName.ExcludeLanguages}
        defaultValue={filters?.excludeLanguages}
        variant="filled"
        searchable
      />
      <Checkbox
        label="Exclude artists already on profile"
        name={FormName.ExcludeKnownArtists}
        defaultValue={filters?.excludeKnownArtists}
        variant="filled"
        value={1}
      />
    </Stack>
  );
};
