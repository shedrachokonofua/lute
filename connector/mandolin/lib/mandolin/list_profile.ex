defmodule Mandolin.ListProfile do
  require Logger

  def ensure_setup(file_name) do
    with {:ok, lookup} <- upsert_lookup(file_name),
         {:ok, profile} <- setup_profile(lookup) do
      {:ok, profile}
    end
  end

  def get_summary(profile_id) do
    with {:ok, summary} <- Mandolin.Lute.Client.get_profile_summary(profile_id) do
      {:ok, summary}
    end
  end

  defp upsert_lookup(file_name) do
    with {:ok, lookup} <-
           Mandolin.Lute.Client.put_list_lookup(file_name) do
      case lookup.status do
        :Completed ->
          {:ok, lookup}

        status when status in [:Started, :InProgress] ->
          {:error, lookup_status: status, progress: lookup_progress(lookup)}

        status ->
          {:error, lookup_status: status}
      end
    end
  end

  defp lookup_progress(lookup) do
    total = Enum.count(lookup.component_processing_statuses)

    completed =
      Enum.count(lookup.component_processing_statuses, fn {_, value} ->
        value == :ReadModelUpdated
      end)

    round(completed / total * 100)
  end

  defp setup_profile(lookup) do
    with {:ok, profile} <- upsert_profile(build_profile_id(lookup.root_file_name)),
         {:ok, profile} <- populate_profile(profile, lookup) do
      {:ok, profile}
    end
  end

  def build_profile_id(file_name) do
    "mandolin_" <> String.replace(file_name, "/", "_")
  end

  defp upsert_profile(profile_id) do
    with {:error, %GRPC.RPCError{status: status}} when status == 5 <-
           Mandolin.Lute.Client.get_profile(profile_id),
         {:ok, profile} <- Mandolin.Lute.Client.create_profile(profile_id, profile_id) do
      Logger.info("Created profile #{profile_id}")
      {:ok, profile}
    end
  end

  defp populate_profile(profile, lookup) do
    missing_albums =
      lookup.segments
      |> Enum.flat_map(& &1.album_file_names)
      |> MapSet.new()
      |> Enum.reject(&Map.has_key?(profile.albums, &1))

    fully_populated = Enum.empty?(missing_albums)

    if fully_populated do
      {:ok, profile}
    else
      input = Enum.map(missing_albums, fn file_name -> %{file_name: file_name, factor: 1} end)
      Logger.info("Populating profile #{profile.id} with #{length(input)} albums")
      Mandolin.Lute.Client.put_many_albums_on_profile(profile.id, input)
    end
  end
end
