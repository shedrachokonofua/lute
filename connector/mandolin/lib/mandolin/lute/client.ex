defmodule Mandolin.Lute.Client do
  require Logger
  alias Mandolin.Lute.Channel

  def get_album_monitor do
    Channel.channel() |> Lute.AlbumService.Stub.get_monitor(%Google.Protobuf.Empty{})
  end


end
