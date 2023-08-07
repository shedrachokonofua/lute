import {
  Button,
  Grid,
  MultiSelect,
  NumberInput,
  Select,
  Stack,
  Title,
} from "@mantine/core";
import { Form } from "react-router-dom";
import { CollapsibleSection } from "../../components";
import {
  GenreAggregate,
  LanguageAggregate,
  Profile,
} from "../../proto/lute_pb";

export type RecommendationMethod = "quantile-ranking";

export const RecommendationSettingsFormName = {
  ProfileId: "profileId",
  Count: "recommendationSettings.count",
  IncludePrimaryGenres: "recommendationSettings.includePrimaryGenres",
  ExcludePrimaryGenres: "recommendationSettings.excludePrimaryGenres",
  IncludeSecondaryGenres: "recommendationSettings.includeSecondaryGenres",
  ExcludeSecondaryGenres: "recommendationSettings.excludeSecondaryGenres",
  IncludeLanguages: "recommendationSettings.includeLanguages",
  ExcludeLanguages: "recommendationSettings.excludeLanguages",
  MinReleaseYear: "recommendationSettings.minReleaseYear",
  MaxReleaseYear: "recommendationSettings.maxReleaseYear",
  Method: "method",
  QuantileRankingPrimaryGenresWeight:
    "assessmentSettings.quantileRanking.primaryGenresWeight",
  QuantileRankingSecondaryGenresWeight:
    "assessmentSettings.quantileRanking.secondaryGenresWeight",
  QuantileRankingDescriptorWeight:
    "assessmentSettings.quantileRanking.descriptorWeight",
  QuantileRankingRatingWeight:
    "assessmentSettings.quantileRanking.ratingWeight",
  QuantileRankingRatingCountWeight:
    "assessmentSettings.quantileRanking.ratingCountWeight",
  QuantileRankingDescriptorCountWeight:
    "assessmentSettings.quantileRanking.descriptorCountWeight",
  QuantileRankingCreditTagWeight:
    "assessmentSettings.quantileRanking.creditTagWeight",
};

export interface RecommendationSettingsForm {
  profileId: string | undefined;
  recommendationSettings:
    | {
        count: number | undefined;
        minReleaseYear: number | undefined;
        maxReleaseYear: number | undefined;
        includePrimaryGenres: string[] | undefined;
        excludePrimaryGenres: string[] | undefined;
        includeSecondaryGenres: string[] | undefined;
        excludeSecondaryGenres: string[] | undefined;
        includeLanguages: string[] | undefined;
        excludeLanguages: string[] | undefined;
      }
    | undefined;
  method: RecommendationMethod | undefined;
  assessmentSettings:
    | {
        quantileRanking:
          | {
              primaryGenresWeight: number | undefined;
              secondaryGenresWeight: number | undefined;
              descriptorWeight: number | undefined;
              ratingWeight: number | undefined;
              ratingCountWeight: number | undefined;
              descriptorCountWeight: number | undefined;
              creditTagWeight: number | undefined;
            }
          | undefined;
      }
    | undefined;
}

export const RecommendationSettings = ({
  profiles,
  aggregatedGenres,
  aggregatedLanguages,
  settings,
}: {
  profiles: Profile[];
  aggregatedGenres: GenreAggregate[];
  aggregatedLanguages: LanguageAggregate[];
  settings: RecommendationSettingsForm | null;
}) => {
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
            />
            <NumberInput
              label="Recommendations Count"
              placeholder="Recommendations Count"
              min={1}
              max={100}
              name={RecommendationSettingsFormName.Count}
              defaultValue={settings?.recommendationSettings?.count || 40}
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
                searchable
              />
            </Stack>
          </CollapsibleSection>
          <Stack spacing="sm">
            <Select
              label="Method"
              data={[{ label: "Quantile Ranking", value: "quantile-ranking" }]}
              defaultValue={settings?.method || "quantile-ranking"}
              placeholder="Select Method"
              name={RecommendationSettingsFormName.Method}
            />
            <CollapsibleSection title="Method Settings">
              <Stack spacing="sm">
                <Title order={6}>Parameter Weights</Title>

                <Grid gutter="xs">
                  <Grid.Col md={6}>
                    <NumberInput
                      label="Primary Genres"
                      placeholder="Primary Genres"
                      min={0}
                      max={20}
                      step={1}
                      name={
                        RecommendationSettingsFormName.QuantileRankingPrimaryGenresWeight
                      }
                      defaultValue={
                        settings?.assessmentSettings?.quantileRanking
                          ?.primaryGenresWeight
                      }
                    />
                  </Grid.Col>
                  <Grid.Col md={6}>
                    <NumberInput
                      label="Secondary Genres"
                      placeholder="Secondary Genres"
                      min={0}
                      max={20}
                      step={1}
                      name={
                        RecommendationSettingsFormName.QuantileRankingSecondaryGenresWeight
                      }
                      defaultValue={
                        settings?.assessmentSettings?.quantileRanking
                          ?.secondaryGenresWeight
                      }
                    />
                  </Grid.Col>
                  <Grid.Col md={6}>
                    <NumberInput
                      label="Descriptor"
                      placeholder="Descriptor"
                      min={0}
                      max={20}
                      step={1}
                      name={
                        RecommendationSettingsFormName.QuantileRankingDescriptorWeight
                      }
                      defaultValue={
                        settings?.assessmentSettings?.quantileRanking
                          ?.descriptorWeight
                      }
                    />
                  </Grid.Col>
                  <Grid.Col md={6}>
                    <NumberInput
                      label="Rating"
                      placeholder="Rating"
                      min={0}
                      max={20}
                      step={1}
                      name={
                        RecommendationSettingsFormName.QuantileRankingRatingWeight
                      }
                      defaultValue={
                        settings?.assessmentSettings?.quantileRanking
                          ?.ratingWeight
                      }
                    />
                  </Grid.Col>
                  <Grid.Col md={6}>
                    <NumberInput
                      label="Rating Count"
                      placeholder="Rating Count"
                      min={0}
                      max={20}
                      step={1}
                      name={
                        RecommendationSettingsFormName.QuantileRankingRatingCountWeight
                      }
                      defaultValue={
                        settings?.assessmentSettings?.quantileRanking
                          ?.ratingCountWeight
                      }
                    />
                  </Grid.Col>
                  <Grid.Col md={6}>
                    <NumberInput
                      label="Descriptor Count"
                      placeholder="Descriptor Count"
                      min={0}
                      max={20}
                      step={1}
                      name={
                        RecommendationSettingsFormName.QuantileRankingDescriptorCountWeight
                      }
                      defaultValue={
                        settings?.assessmentSettings?.quantileRanking
                          ?.descriptorCountWeight
                      }
                    />
                  </Grid.Col>
                  <Grid.Col md={6}>
                    <NumberInput
                      label="Credits"
                      placeholder="Credits"
                      min={0}
                      max={20}
                      step={1}
                      name={
                        RecommendationSettingsFormName.QuantileRankingCreditTagWeight
                      }
                      defaultValue={
                        settings?.assessmentSettings?.quantileRanking
                          ?.creditTagWeight
                      }
                    />
                  </Grid.Col>
                </Grid>
              </Stack>
            </CollapsibleSection>
          </Stack>
          <div>
            <Button
              style={{
                background: "#2D78A0",
              }}
              type="submit"
              fullWidth
            >
              Submit
            </Button>
          </div>
        </Stack>
      </Form>
    </Stack>
  );
};
