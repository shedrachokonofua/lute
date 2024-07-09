defmodule Mandolin.Recommendation do
  alias Mandolin.Recommendation.Settings
  alias Mandolin.ListProfile

  def albums(file_name, %Settings{} = settings) do
    assessment_settings =
      %Lute.AlbumAssessmentSettings{
        settings:
          if settings.style == "safe" do
            {
              :quantile_rank_settings,
              %Lute.QuantileRankAlbumAssessmentSettings{}
            }
          else
            {
              :embedding_similarity_settings,
              %Lute.EmbeddingSimilarityAlbumAssessmentSettings{
                embedding_key: "voyageai-default"
              }
            }
          end
      }

    recommendation_settings = %Lute.AlbumRecommendationSettings{
      count: 5,
      min_release_year: settings.min_year,
      max_release_year: settings.max_year
    }

    Mandolin.Lute.Client.recommend_albums(
      ListProfile.build_profile_id(file_name),
      recommendation_settings,
      assessment_settings
    )
  end
end
