defmodule Mandolin.Lute.Client do
  alias Mandolin.Lute.Channel

  def get_profile(id) do
    with {:ok, %Lute.GetProfileReply{profile: profile}} <-
           Lute.ProfileService.Stub.get_profile(Channel.channel(), %Lute.GetProfileRequest{id: id}) do
      {:ok, profile}
    end
  end

  def get_profile_summary(id) do
    with {:ok, %Lute.GetProfileSummaryReply{summary: summary}} <-
           Lute.ProfileService.Stub.get_profile_summary(
             Channel.channel(),
             %Lute.GetProfileSummaryRequest{id: id}
           ) do
      {:ok, summary}
    end
  end

  def create_profile(id, name) do
    with {:ok, %Lute.CreateProfileReply{profile: profile}} <-
           Lute.ProfileService.Stub.create_profile(Channel.channel(), %Lute.CreateProfileRequest{
             id: id,
             name: name
           }) do
      {:ok, profile}
    end
  end

  def put_list_lookup(file_name) do
    with {:ok, %Lute.PutListLookupReply{lookup: lookup}} <-
           Lute.LookupService.Stub.put_list_lookup(Channel.channel(), %Lute.PutListLookupRequest{
             file_name: file_name
           }) do
      {:ok, lookup}
    end
  end

  def put_many_albums_on_profile(profile_id, albums) do
    request_albums =
      Enum.map(albums, fn album ->
        %Lute.FileNameWithFactor{file_name: album.file_name, factor: album.factor}
      end)

    with {:ok, %Lute.PutManyAlbumsOnProfileReply{profile: profile}} <-
           Lute.ProfileService.Stub.put_many_albums_on_profile(
             Channel.channel(),
             %Lute.PutManyAlbumsOnProfileRequest{
               profile_id: profile_id,
               albums: request_albums
             }
           ) do
      {:ok, profile}
    end
  end
end
