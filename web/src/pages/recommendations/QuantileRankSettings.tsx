import { Grid, NumberInput, Stack, Title } from "@mantine/core";
import { FormName } from "../../forms";
import { QuantileRankAlbumAssessmentSettings } from "../../proto/lute_pb";
import { RecommendationSettingsForm } from "./types";

export const QuantileRankSettings = ({
  settings,
  defaultQuantileRankAlbumAssessmentSettings,
}: {
  settings: RecommendationSettingsForm | null;
  defaultQuantileRankAlbumAssessmentSettings: QuantileRankAlbumAssessmentSettings;
}) => (
  <Stack gap="sm">
    <Title order={6}>Parameter Weights</Title>

    <Grid gutter="xs">
      <Grid.Col span={6}>
        <NumberInput
          label="Primary Genres"
          placeholder="Primary Genres"
          min={0}
          max={20}
          step={1}
          name={FormName.QuantileRankingPrimaryGenresWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking
              ?.primaryGenresWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getPrimaryGenreWeight()
          }
          variant="filled"
        />
      </Grid.Col>
      <Grid.Col span={6}>
        <NumberInput
          label="Secondary Genres"
          placeholder="Secondary Genres"
          min={0}
          max={20}
          step={1}
          name={FormName.QuantileRankingSecondaryGenresWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking
              ?.secondaryGenresWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getSecondaryGenreWeight()
          }
          variant="filled"
        />
      </Grid.Col>
      <Grid.Col span={6}>
        <NumberInput
          label="Descriptor"
          placeholder="Descriptor"
          min={0}
          max={20}
          step={1}
          name={FormName.QuantileRankingDescriptorWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking?.descriptorWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getDescriptorWeight()
          }
          variant="filled"
        />
      </Grid.Col>
      <Grid.Col span={6}>
        <NumberInput
          label="Rating"
          placeholder="Rating"
          min={0}
          max={20}
          step={1}
          name={FormName.QuantileRankingRatingWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking?.ratingWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getRatingWeight()
          }
          variant="filled"
        />
      </Grid.Col>
      <Grid.Col span={6}>
        <NumberInput
          label="Rating Count"
          placeholder="Rating Count"
          min={0}
          max={20}
          step={1}
          name={FormName.QuantileRankingRatingCountWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking?.ratingCountWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getRatingCountWeight()
          }
          variant="filled"
        />
      </Grid.Col>
      <Grid.Col span={6}>
        <NumberInput
          label="Descriptor Count"
          placeholder="Descriptor Count"
          min={0}
          max={20}
          step={1}
          name={FormName.QuantileRankingDescriptorCountWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking
              ?.descriptorCountWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getDescriptorCountWeight()
          }
          variant="filled"
        />
      </Grid.Col>
      <Grid.Col span={6}>
        <NumberInput
          label="Credits"
          placeholder="Credits"
          min={0}
          max={20}
          step={1}
          name={FormName.QuantileRankingCreditTagWeight}
          defaultValue={
            settings?.assessmentSettings?.quantileRanking?.creditTagWeight ??
            defaultQuantileRankAlbumAssessmentSettings.getCreditTagWeight()
          }
          variant="filled"
        />
      </Grid.Col>
    </Grid>
  </Stack>
);
