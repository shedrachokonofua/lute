import {
  AlbumServiceClient,
  FileServiceClient,
  ProfileServiceClient,
  RecommendationServiceClient,
} from "./proto/LuteServiceClientPb";
import {
  Album,
  AssessAlbumRequest,
  DeleteFileRequest,
  GetFilePageTypeRequest,
  Profile,
  PutFileRequest,
  FindSimilarAlbumsRequest,
  GetAlbumRequest,
  AlbumSearchQuery,
} from "./proto/lute_pb";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";

const url = "http://localhost:22000";
const client = {
  file: new FileServiceClient(url),
  profile: new ProfileServiceClient(url),
  recommendation: new RecommendationServiceClient(url),
  album: new AlbumServiceClient(url),
};

export const putFile = async (fileName: string, content: string) => {
  const putFileRequest = new PutFileRequest();
  putFileRequest.setName(fileName);
  putFileRequest.setContent(content);

  await client.file.putFile(putFileRequest, {});
};

export const getFilePageType = async (fileName: string) => {
  const getFilePageTypeRequest = new GetFilePageTypeRequest();
  getFilePageTypeRequest.setName(fileName);

  const reply = await client.file.getFilePageType(getFilePageTypeRequest, {});
  return reply.getPageType();
};

export const getAllProfiles = async (): Promise<Profile[]> => {
  const response = await client.profile.getAllProfiles(new Empty(), null);
  return response.getProfilesList();
};

export const assessAlbum = async (profileId: string, fileName: string) => {
  const request = new AssessAlbumRequest();
  request.setFileName(fileName);
  request.setProfileId(profileId);
  const reply = await client.recommendation.assessAlbum(request, null);
  return reply.getAssessment();
};

export const deleteFile = async (fileName: string) => {
  const request = new DeleteFileRequest();
  request.setName(fileName);
  await client.file.deleteFile(request, null);
};

export const findAlbum = async (
  fileName: string
): Promise<Album | undefined> => {
  const request = new GetAlbumRequest();
  request.setFileName(fileName);
  try {
    const response = await client.album.getAlbum(request, null);
    return response.getAlbum();
  } catch (e) {
    console.error(e);
    return undefined;
  }
};

export const findSimilarAlbums = async (
  fileName: string,
  limit = 5
): Promise<Album[]> => {
  const request = new FindSimilarAlbumsRequest();
  if (!fileName) {
    throw new Error("Invalid settings");
  }
  request.setFileName(fileName);
  request.setEmbeddingKey("voyageai-default");

  if (limit) {
    request.setLimit(limit);
  }

  const response = await client.album.findSimilarAlbums(request, null);
  return response.getAlbumsList();
};
