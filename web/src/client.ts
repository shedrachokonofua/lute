import { LuteClient, SpotifyServiceClient } from "./proto/LuteServiceClientPb";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { HandleAuthorizationCodeRequest } from "./proto/lute_pb";

const coreUrl = "http://localhost:22000";
const spotifyClient = new SpotifyServiceClient(coreUrl);

export const isSpotifyAuthorized = async (): Promise<boolean> => {
  const response = await spotifyClient.isAuthorized(new Empty(), null);
  return response.getAuthorized();
};

export const getSpotifyAuthUrl = async (): Promise<string> => {
  const response = await spotifyClient.getAuthorizationUrl(new Empty(), null);
  return response.getUrl();
};

export const handleSpotifyAuthCode = async (code: string): Promise<void> => {
  const request = new HandleAuthorizationCodeRequest();
  request.setCode(code);
  await spotifyClient.handleAuthorizationCode(request, null);
};
