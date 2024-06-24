defmodule Mandolin.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      MandolinWeb.Telemetry,
      {DNSCluster, query: Application.get_env(:mandolin, :dns_cluster_query) || :ignore},
      {Phoenix.PubSub, name: Mandolin.PubSub},
      # Start the Finch HTTP client for sending emails
      {Finch, name: Mandolin.Finch},
      # Start a worker by calling: Mandolin.Worker.start_link(arg)
      # {Mandolin.Worker, arg},
      # Start to serve requests, typically the last entry
      MandolinWeb.Endpoint
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: Mandolin.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    MandolinWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
