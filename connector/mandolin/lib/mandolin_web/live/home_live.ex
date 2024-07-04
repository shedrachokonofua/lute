defmodule MandolinWeb.HomeLive do
  use MandolinWeb, :live_view

  def mount(_params, _session, socket) do
    form = to_form(%{"url" => ""})
    {:ok, assign(socket, form: form, form_valid: false)}
  end

  def handle_event("validate", %{"url" => url}, socket) do
    case validate_url(url) do
      {:ok, _} ->
        {:noreply, assign(socket, form: to_form(%{"url" => url}), form_valid: true)}

      {:error, reason} ->
        {:noreply,
         assign(socket,
           form: to_form(%{"url" => url}, errors: [url: {reason, []}]),
           form_valid: false
         )}
    end
  end

  def handle_event("save", %{"url" => url}, socket) do
    case get_file_name(url) do
      {:ok, file_name} ->
        {:noreply, redirect(socket, to: file_name)}

      {:error, reason} ->
        {:noreply,
         assign(socket,
           form: to_form(%{"url" => url}, errors: [url: {reason, []}]),
           form_valid: false
         )}
    end
  end

  defp get_file_name(url) do
    with {:ok, url} <- validate_url(url) do
      full_file_name = Regex.replace(~r/^(https?:\/\/)?(www\.)?rateyourmusic\.com\//, url, "")
      parts = String.split(full_file_name, "/")
      file_name = "/" <> Enum.join(Enum.take(parts, 3), "/")
      {:ok, file_name}
    end
  end

  defp validate_url(url) do
    if ~r{^(https?:\/\/)?(www\.)?rateyourmusic\.com\/list\/[^\/]+\/[^\/]+(\/\d+)?\/?$}
       |> Regex.match?(url) do
      {:ok, url}
    else
      {:error, "Invalid URL"}
    end
  end
end
