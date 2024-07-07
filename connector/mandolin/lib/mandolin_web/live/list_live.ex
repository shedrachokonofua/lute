defmodule MandolinWeb.ListLive do
  require Logger
  alias Phoenix.LiveView.AsyncResult
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
            <% [status: status, progress: progress] when status in [:Started, :InProgress] -> %>
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
          <.recommendations_tab page_data={page_data} />
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
      <div>
        <article class="rounded-xl bg-gradient-to-r from-green-300 via-blue-500 to-purple-600 p-0.5 shadow-lg">
          <div class="rounded-[10px] bg-white p-4 flex flex-col gap-4">
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
                      id="horizontal-list-radio-safe"
                      type="radio"
                      value="safe"
                      name="recommendation-style"
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
                      id="horizontal-list-radio-adventurous"
                      type="radio"
                      value="adventurous"
                      name="recommendation-style"
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
                  <.input name="min_year" value="1900" label="Minimum" type="number" class="w-full" />
                </div>
                <div>
                  <.input name="max_year" value="2024" label="Maximum" type="number" class="w-full" />
                </div>
              </div>
            </div>
            <div>
              <.button type="submit" class="w-full mt-4">
                Submit
              </.button>
            </div>
          </div>
        </article>
      </div>
      <div class="lg:col-span-2">Right</div>
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
        _params,
        _session,
        socket
      ) do
    {:ok, assign(socket, tab_labels: @tab_labels, page_data: AsyncResult.loading())}
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
    with {:ok, lookup} <- fetch_lookup(file_name),
         {:ok, profile} <- setup_profile(lookup),
         {:ok, profile_summary} <- Mandolin.Lute.Client.get_profile_summary(profile.id) do
      {:ok, %{page_data: %{lookup: lookup, profile: profile, profile_summary: profile_summary}}}
    end
  end

  defp fetch_lookup(file_name) do
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
    profile_id = "mandolin_" <> String.replace(lookup.root_file_name, "/", "_")

    with {:ok, profile} <- upsert_profile(profile_id),
         {:ok, profile} <- populate_profile(profile, lookup) do
      {:ok, profile}
    end
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
    populated =
      lookup.segments
      |> Enum.flat_map(& &1.album_file_names)
      |> MapSet.new()
      |> Enum.all?(&Map.has_key?(profile.albums, &1))

    if populated do
      {:ok, profile}
    else
      input =
        Enum.flat_map(lookup.segments, fn segment ->
          Enum.map(segment.album_file_names, fn file_name ->
            %{file_name: file_name, factor: 1}
          end)
        end)

      Logger.info("Populating profile #{profile.id} with #{length(input)} albums")
      Mandolin.Lute.Client.put_many_albums_on_profile(profile.id, input)
    end
  end
end
