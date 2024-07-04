defmodule Mandolin.Lute.Channel do
  use GenServer
  require Logger

  # Client
  def start_link(_) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  def channel() do
    GenServer.call(__MODULE__, :channel)
  end

  # Server
  @impl true
  def init(_) do
    {:ok, connect_channel()}
  end

  @impl true
  def handle_call(:channel, _from, channel) do
    {:reply, channel, channel}
  end

  @impl true
  def handle_info({:gun_down, _, _, _, _}, _state) do
    Logger.info("GRPC Server disconnected. Reconnecting...")
    {:noreply, connect_channel()}
  end

  defp connect_channel() do
    address = Application.get_env(:mandolin, :core_url)
    Logger.info("GRPC Client connecting to gateway at #{address}")

    case GRPC.Stub.connect(address, interceptors: [GRPC.Client.Interceptors.Logger]) do
      {:ok, channel} ->
        Logger.info("GRPC Client connected to the gateway at #{address}")
        channel

      {:error, error} ->
        Logger.error("GRPC Client could not connect to GRPC Server. Message: #{error}")
        raise "Failed to connect to GRPC server: #{error}"
    end
  end
end
