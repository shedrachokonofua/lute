defmodule MandolinWeb.ListLive do
  use MandolinWeb, :live_view

  @tab_labels [
    recommendations: "Recommendations",
    albums: "List"
  ]

  def mount(
        params,
        _session,
        socket
      ) do
    tab = params |> Map.get("tab", "recommendations") |> sanitize_tab()
    file_name = "list/#{params["user"]}/#{params["list_name"]}"

    {:ok,
     assign(socket,
       file_name: file_name,
       tab: String.to_existing_atom(tab),
       tab_labels: @tab_labels
     )}
  end

  defp sanitize_tab(tab) do
    if Keyword.has_key?(@tab_labels, String.to_existing_atom(tab)) do
      tab
    else
      "recommendations"
    end
  end
end
