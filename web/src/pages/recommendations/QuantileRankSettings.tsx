import { Grid, NumberInput, Stack, Title } from "@mantine/core";
import { QuantileRankAlbumAssessmentSettings } from "../../proto/lute_pb";
import {
  RecommendationSettingsForm,
  RecommendationSettingsFormName,
} from "./types";

export const QuantileRankSettings = ({
  settings,
  defaultQuantileRankAlbumAssessmentSettings,
}: {
  settings: RecommendationSettingsForm | null;
  defaultQuantileRankAlbumAssessmentSettings: QuantileRankAlbumAssessmentSettings;
}) => (
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
              ?.primaryGenresWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getPrimaryGenreWeight()
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
              ?.secondaryGenresWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getSecondaryGenreWeight()
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
          name={RecommendationSettingsFormName.QuantileRankingDescriptorWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking?.descriptorWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getDescriptorWeight()
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
          name={RecommendationSettingsFormName.QuantileRankingRatingWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking?.ratingWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getRatingWeight()
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
          name={RecommendationSettingsFormName.QuantileRankingRatingCountWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking?.ratingCountWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getRatingCountWeight()
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
              ?.descriptorCountWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getDescriptorCountWeight()
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
          name={RecommendationSettingsFormName.QuantileRankingCreditTagWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking?.creditTagWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getCreditTagWeight()
          }
        />
      </Grid.Col>
    </Grid>
  </Stack>
);
