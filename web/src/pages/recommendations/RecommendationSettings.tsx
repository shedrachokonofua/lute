import {
  Button,
  Checkbox,
  Grid,
  MultiSelect,
  NumberInput,
  Select,
  Stack,
  Title,
} from "@mantine/core";
import { useState } from "react";
import { Form } from "react-router-dom";
import { CollapsibleSection } from "../../components";
import {
  GenreAggregate,
  LanguageAggregate,
  Profile,
  QuantileRankAlbumAssessmentSettings,
} from "../../proto/lute_pb";
import { EmbeddingSimilaritySettings } from "./EmbeddingSimilaritySettings";
import { QuantileRankSettings } from "./QuantileRankSettings";
import {
  RecommendationSettingsForm,
  RecommendationSettingsFormName,
} from "./types";

export const RecommendationSettings = ({
  profiles,
  aggregatedGenres,
  aggregatedLanguages,
  embeddingKeys,
  settings,
  defaultQuantileRankAlbumAssessmentSettings,
}: {
  profiles: Profile[];
  aggregatedGenres: GenreAggregate[];
  aggregatedLanguages: LanguageAggregate[];
  embeddingKeys: string[];
  settings: RecommendationSettingsForm | null;
  defaultQuantileRankAlbumAssessmentSettings: QuantileRankAlbumAssessmentSettings;
}) => {
  const [currentMethod, setCurrentMethod] = useState<string>(
    settings?.method || "quantile-ranking",
  );
  const profileOptions = profiles.map((profile) => ({
    label: profile.getName(),
    value: profile.getId(),
  }));
  const primaryGenreOptions = aggregatedGenres.map((genre) => ({
    label: `${genre.getName()} (${genre.getPrimaryGenreCount()})`,
    value: genre.getName(),
  }));
  const secondaryGenreOptions = aggregatedGenres.map((genre) => ({
    label: `${genre.getName()} (${genre.getSecondaryGenreCount()})`,
    value: genre.getName(),
  }));
  const languageOptions = aggregatedLanguages.map((language) => ({
    label: `${language.getName()} (${language.getCount()})`,
    value: language.getName(),
  }));

  return (
    <Stack spacing="lg">
      <Title order={4}>Settings</Title>
      <Form role="search">
        <Stack spacing="xl">
          <Stack spacing="sm">
            <Select
              label="Profile"
              data={profileOptions}
              placeholder="Select Profile"
              name={RecommendationSettingsFormName.ProfileId}
              defaultValue={settings?.profileId}
              variant="filled"
            />
            <NumberInput
              label="Recommendations Count"
              placeholder="Recommendations Count"
              min={1}
              max={100}
              name={RecommendationSettingsFormName.Count}
              defaultValue={settings?.recommendationSettings?.count || 40}
              variant="filled"
            />
          </Stack>
          <CollapsibleSection title="Filters">
            <Stack spacing="sm">
              <Grid gutter="xs">
                <Grid.Col md={6}>
                  <NumberInput
                    label="Min Release Year"
                    placeholder="Year"
                    min={1900}
                    max={2023}
                    step={1}
                    name={RecommendationSettingsFormName.MinReleaseYear}
                    defaultValue={
                      settings?.recommendationSettings?.minReleaseYear
                    }
                    variant="filled"
                  />
                </Grid.Col>
                <Grid.Col md={6}>
                  <NumberInput
                    label="Max Release Year"
                    placeholder="Year"
                    min={1900}
                    max={2023}
                    step={1}
                    name={RecommendationSettingsFormName.MaxReleaseYear}
                    defaultValue={
                      settings?.recommendationSettings?.maxReleaseYear
                    }
                    variant="filled"
                  />
                </Grid.Col>
              </Grid>
              <MultiSelect
                label="Include Primary Genres"
                data={primaryGenreOptions}
                placeholder="Select Genres"
                name={RecommendationSettingsFormName.IncludePrimaryGenres}
                defaultValue={
                  settings?.recommendationSettings?.includePrimaryGenres
                }
                variant="filled"
                searchable
              />
              <MultiSelect
                label="Exclude Primary Genres"
                data={primaryGenreOptions}
                placeholder="Select Genres"
                name={RecommendationSettingsFormName.ExcludePrimaryGenres}
                defaultValue={
                  settings?.recommendationSettings?.excludePrimaryGenres
                }
                variant="filled"
                searchable
              />
              <MultiSelect
                label="Include Secondary Genres"
                data={secondaryGenreOptions}
                placeholder="Select Genres"
                name={RecommendationSettingsFormName.IncludeSecondaryGenres}
                defaultValue={
                  settings?.recommendationSettings?.includeSecondaryGenres
                }
                variant="filled"
                searchable
              />
              <MultiSelect
                label="Exclude Secondary Genres"
                data={secondaryGenreOptions}
                placeholder="Select Genres"
                name={RecommendationSettingsFormName.ExcludeSecondaryGenres}
                defaultValue={
                  settings?.recommendationSettings?.excludeSecondaryGenres
                }
                variant="filled"
                searchable
              />
              <MultiSelect
                label="Include Languages"
                data={languageOptions}
                placeholder="Select Languages"
                name={RecommendationSettingsFormName.IncludeLanguages}
                defaultValue={
                  settings?.recommendationSettings?.includeLanguages
                }
                variant="filled"
                searchable
              />
              <MultiSelect
                label="Exclude Languages"
                data={languageOptions}
                placeholder="Select Languages"
                name={RecommendationSettingsFormName.ExcludeLanguages}
                defaultValue={
                  settings?.recommendationSettings?.excludeLanguages
                }
                variant="filled"
                searchable
              />
              <Checkbox
                label="Exclude artists already on profile"
                name={RecommendationSettingsFormName.ExcludeKnownArtists}
                defaultValue={
                  settings?.recommendationSettings?.excludeKnownArtists
                }
                variant="filled"
                value={1}
              />
            </Stack>
          </CollapsibleSection>
          <Stack spacing="sm">
            <Select
              label="Method"
              data={[
                { label: "Quantile Ranking", value: "quantile-ranking" },
                {
                  label: "Embedding Similarity",
                  value: "embedding-similarity",
                },
              ]}
              value={currentMethod}
              onChange={(value) => {
                if (value) {
                  setCurrentMethod(value);
                }
              }}
              placeholder="Select Method"
              name={RecommendationSettingsFormName.Method}
              variant="filled"
            />
            <CollapsibleSection title="Method Settings">
              {currentMethod === "quantile-ranking" && (
                <QuantileRankSettings
                  settings={settings}
                  defaultQuantileRankAlbumAssessmentSettings={
                    defaultQuantileRankAlbumAssessmentSettings
                  }
                />
              )}
              {currentMethod === "embedding-similarity" && (
                <EmbeddingSimilaritySettings
                  settings={settings}
                  embbedingKeys={embeddingKeys}
                />
              )}
            </CollapsibleSection>
          </Stack>
          <div>
            <Button type="submit" fullWidth>
              Submit
            </Button>
          </div>
        </Stack>
      </Form>
    </Stack>
  );
};
