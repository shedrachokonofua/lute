defmodule MandolinWeb.ListLive do
  require Logger
  alias Mandolin.Recommendation
  alias Mandolin.ListProfile
  alias Phoenix.LiveView.AsyncResult
  alias Mandolin.Lute.Client
  use MandolinWeb, :live_view

  @tab_labels [
    recommendations: "Recommendations",
    albums: "List"
  ]
  @valid_tabs Keyword.keys(@tab_labels)

  def render(assigns) do
    ~H"""
    <.flash_group flash={@flash} />
    <.async_result :let={page_data} assign={@page_data}>
      <:loading><.loader /></:loading>
      <:failed :let={{:error, error}}>
        <div>
          <%= case error do %>
            <% [lookup_status: status, progress: progress] when status in [:Started, :InProgress] -> %>
              <.in_progress_message progress={progress} />
            <% _ -> %>
              <.failure_message error={error} />
          <% end %>
        </div>
      </:failed>
      <div>
        <div class="py-6">
          <h2>
            <%= @file_name %>
          </h2>
        </div>
        <.nav_bar file_name={@file_name} current_tab={@tab} tab_labels={@tab_labels} />
        <%= if @tab == :albums do %>
          <.albums_tab page_data={page_data} />
        <% else %>
          <.recommendations_tab
            page_data={page_data}
            recommendation_settings={@recommendation_settings}
            recommendation_data={@recommendation_data}
          />
        <% end %>
      </div>
    </.async_result>
    """
  end

  defp loader(assigns) do
    ~H"""
    <div class="flex justify-center items-center p-20">
      <.spinner />
    </div>
    """
  end

  defp nav_bar(assigns) do
    ~H"""
    <nav class="mb-6">
      <ul class="flex flex-wrap text-sm font-medium text-center text-gray-500 border-b border-gray-200 dark:border-gray-700 dark:text-gray-400">
        <%= for {tab_key, label} <- @tab_labels do %>
          <li class="me-2">
            <.link
              patch={"/#{@file_name}/#{tab_key}"}
              class={[
                "inline-block p-3",
                @current_tab == tab_key &&
                  "text-brand bg-brand/10 active dark:bg-gray-800 dark:text-blue-500",
                @current_tab != tab_key &&
                  "hover:text-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800 dark:hover:text-gray-300"
              ]}
              aria-current={@current_tab == tab_key && "page"}
            >
              <%= label %>
            </.link>
          </li>
        <% end %>
      </ul>
    </nav>
    """
  end

  defp in_progress_message(assigns) do
    ~H"""
    <p>
      We're retrieving and analyzing your list! It's about <%= @progress %>% complete. This may take a few minutes.
    </p>
    """
  end

  defp failure_message(assigns) do
    ~H"""
    <p>
      <%= case assigns.error do %>
        <% [status: :Invalid] -> %>
          Invalid request, this list likely doesn't exist.
        <% [status: :Failed] -> %>
          Failed to analyze the list, we're sorry! We'll look into it.
        <% _ -> %>
          An unexpected error occurred, we're sorry! We'll look into it.
      <% end %>
    </p>
    """
  end

  defp recommendations_tab(assigns) do
    ~H"""
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-3 lg:gap-8">
      <div class="border-right border-gray-200 border-2">
        <article class="rounded-xl p-0.5">
          <.form for={%{}} phx-submit="update_recommendations">
            <div class="p-4 flex flex-col gap-4">
              <div class="flex items-center gap-2">
                <.icon name="hero-cog-6-tooth" />
                <h2 class="mt-0.5 font-semibold text-brand">
                  Settings
                </h2>
              </div>
              <div>
                <p class="text-xs text-gray-600">
                  Update your preferences to get tailored recommendations.
                </p>
              </div>
              <div>
                <h3 class="font-medium text-gray-900 dark:text-white">
                  Recommendation Style
                </h3>
                <p class="text-xs text-gray-600 dark:text-gray-400 mb-2">
                  Choose how adventurous you want your music recommendations to be.
                </p>
                <ul class="w-full text-sm font-medium text-gray-900 bg-white border border-gray-200 rounded-lg sm:flex dark:bg-gray-700 dark:border-gray-600 dark:text-white">
                  <li class="w-full border-b border-gray-200 sm:border-b-0 sm:border-r dark:border-gray-600">
                    <div class="flex items-center ps-3 py-3">
                      <input
                        checked={@recommendation_settings.style == "safe"}
                        id="horizontal-list-radio-safe"
                        type="radio"
                        value="safe"
                        name="style"
                        class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-700 dark:focus:ring-offset-gray-700 focus:ring-2 dark:bg-gray-600 dark:border-gray-500"
                      />
                      <label
                        for="horizontal-list-radio-safe"
                        class="ms-2 text-sm font-medium text-gray-900 dark:text-gray-300"
                      >
                        Safe
                      </label>
                    </div>
                    <p class="px-3 pb-3 text-sm text-gray-600 dark:text-gray-400">
                      Receive recommendations that favor well rated and annotated albums.
                    </p>
                  </li>
                  <li class="w-full dark:border-gray-600">
                    <div class="flex items-center ps-3 py-3">
                      <input
                        checked={@recommendation_settings.style == "adventurous"}
                        id="horizontal-list-radio-adventurous"
                        type="radio"
                        value="adventurous"
                        name="style"
                        class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-700 dark:focus:ring-offset-gray-700 focus:ring-2 dark:bg-gray-600 dark:border-gray-500"
                      />
                      <label
                        for="horizontal-list-radio-adventurous"
                        class="ms-2 text-sm font-medium text-gray-900 dark:text-gray-300"
                      >
                        Adventurous
                      </label>
                    </div>
                    <p class="px-3 pb-3 text-sm text-gray-600 dark:text-gray-400">
                      Explore unique and diverse recommendations not influenced by popularity, ratings, or annotation quality.
                    </p>
                  </li>
                </ul>
              </div>
              <div>
                <h3 class="font-medium text-gray-900 dark:text-white">
                  Release Year
                </h3>
                <p class="text-xs text-gray-600 dark:text-gray-400 mb-2">
                  Set the range of release years for your recommendations.
                </p>
                <div class="flex items-center gap-2">
                  <div>
                    <.input
                      name="min_year"
                      value={@recommendation_settings.min_year}
                      label="Minimum"
                      type="number"
                      class="w-full"
                    />
                  </div>
                  <div>
                    <.input
                      name="max_year"
                      value={@recommendation_settings.max_year}
                      label="Maximum"
                      type="number"
                      class="w-full"
                    />
                  </div>
                </div>
              </div>
              <div>
                <.button type="submit" class="w-full mt-4">
                  Submit
                </.button>
              </div>
            </div>
          </.form>
        </article>
      </div>
      <div class="lg:col-span-2">
        <.async_result :let={data} assign={@recommendation_data}>
          <:loading>
            <div class="flex flex-col gap-4">
              <div :for={_ <- 1..5} class="bg-brand/25 animate-pulse" style="height: 75px"></div>
            </div>
          </:loading>
          <:failed>
            <div>Failed to load recommendations</div>
          </:failed>
          <div class="flex flex-col divide-y divide-slate-200 shadow-xs rounded-lg shadow-brand">
            <div class="p-2">
              <h2 class="font-semibold text-brand">
                Recommendations
              </h2>
            </div>
            <div :for={r <- data} class="flex gap-4 p-4">
              <img
                src={r.album.cover_image_url}
                alt={r.album.name}
                width="75"
                style="height: 75px"
                id={"cover-" <> r.album.file_name}
                phx-hook="AlbumCover"
                data-filename={r.album.file_name}
              />
              <div>
                <div class="font-medium text-brand">
                  <%= r.album.name %> (<%= r.album.release_date %>)
                </div>
                <div class="text-sm">
                  <%= Enum.map(r.album.artists, & &1.name) |> Enum.join(", ") %>
                </div>
                <div class="text-sm"><%= r.album.primary_genres |> Enum.join(", ") %></div>
                <div>
                  <div class="w-3xl pt-4">
                    <div data-spotifyid={r.album.spotify_id} class="spotify-player"></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </.async_result>
      </div>
    </div>
    """
  end

  defp albums_tab(assigns) do
    ~H"""
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-3 lg:gap-8">
      <div class="">Left</div>
      <div class="lg:col-span-2">Right</div>
    </div>
    """
  end

  def mount(
        params,
        _session,
        socket
      ) do
    {:ok,
     assign(socket,
       tab_labels: @tab_labels,
       page_data: AsyncResult.loading(),
       recommendation_settings: Recommendation.Settings.build(params),
       recommendation_data: AsyncResult.loading()
     )}
  end

  def handle_params(
        params,
        _uri,
        socket
      ) do
    tab = get_tab(params)
    file_name = "list/#{params["user"]}/#{params["list_name"]}"

    socket =
      if connected?(socket) do
        assign_async(socket, :page_data, fn -> fetch_data(file_name) end)
      else
        socket
      end

    socket =
      if connected?(socket) do
        case tab do
          :recommendations ->
            recommendation_settings = Recommendation.Settings.build(params)

            socket
            |> assign(:recommendation_settings, recommendation_settings)
            |> assign_async(:recommendation_data, fn ->
              fetch_recommendations(file_name, recommendation_settings)
            end)

          _ ->
            socket
            |> assign(:recommendation_data, AsyncResult.loading())
        end
      else
        socket
      end

    {:noreply,
     assign(socket,
       file_name: file_name,
       tab: tab,
       tab_labels: @tab_labels
     )}
  end

  defp get_tab(%{"tab" => tab}) do
    tab
    |> String.to_existing_atom()
    |> sanitize_tab()
  end

  defp get_tab(_), do: :recommendations

  defp sanitize_tab(tab) when tab in @valid_tabs, do: tab
  defp sanitize_tab(_), do: :recommendations

  defp fetch_data(file_name) do
    with {:ok, profile} <- ListProfile.ensure_setup(file_name),
         {:ok, profile_summary} <- ListProfile.get_summary(profile.id) do
      {:ok, %{page_data: %{profile: profile, profile_summary: profile_summary}}}
    end
  end

  defp fetch_recommendations(file_name, settings) do
    with {:ok, recommendations} <- Recommendation.albums(file_name, settings) do
      {:ok, %{recommendation_data: recommendations}}
    end
  end

  def handle_event("update_recommendations", params, socket) do
    settings = Recommendation.Settings.build(params)

    next =
      "/#{socket.assigns.file_name}/#{socket.assigns.tab}?#{URI.encode_query(Map.from_struct(settings))}"

    {:noreply,
     socket |> assign(:recommendation_data, AsyncResult.loading()) |> push_patch(to: next)}
  end

  def handle_event("album_cover_error", %{"file_name" => file_name}, socket) do
    Logger.info("Failed to load cover for #{file_name}, enqueueing crawl")
    Client.crawl(file_name)
    {:noreply, socket}
  end
end
