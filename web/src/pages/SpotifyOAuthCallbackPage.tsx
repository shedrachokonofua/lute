import { Async } from "react-async";
import { handleSpotifyAuthCode } from "../client";

const onLoad = async () => {
  const query = new URLSearchParams(window.location.search);
  const code = query.get("code");
  if (!code) {
    throw new Error("No code provided");
  }
  await handleSpotifyAuthCode(code);
  window.location.assign("/");
};

export const SpotifyOAuthCallbackPage = () => (
  <Async promiseFn={onLoad}>
    <Async.Loading>Loading...</Async.Loading>
    <Async.Rejected>{(error) => <div>{error.message}</div>}</Async.Rejected>
  </Async>
);
