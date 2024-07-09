defmodule Mandolin.Recommendation.Settings do
  require Logger

  defstruct style: "safe", min_year: 1900, max_year: 2024

  defmodule RecommendationSettingsContract do
    use Drops.Contract

    schema(atomize: true) do
      %{
        optional(:style) => string(in?: ["safe", "adventurous"]),
        optional(:min_year) => cast(string(match?: ~r/\d+/)) |> integer(),
        optional(:max_year) => cast(string(match?: ~r/\d+/)) |> integer()
      }
    end

    rule(:min_lt_max_year, %{min_year: min_year, max_year: max_year}) do
      if min_year > max_year do
        {:error, {[:min_year, :max_year], "Min year must be less than max year"}}
      else
        :ok
      end
    end
  end

  def build(params) do
    case RecommendationSettingsContract.conform(params) do
      {:ok, data} ->
        struct!(__MODULE__, data)

      {:error, e} ->
        Logger.warning("Invalid recommendation settings: #{inspect(e)}, using defaults")
        %__MODULE__{}
    end
  end
end
