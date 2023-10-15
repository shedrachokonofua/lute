import {
  FileServiceClient,
  ProfileServiceClient,
  RecommendationServiceClient,
} from "./proto/LuteServiceClientPb";
import {
  AssessAlbumRequest,
  GetFilePageTypeRequest,
  Profile,
  PutFileRequest,
} from "./proto/lute_pb";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";

const url = "http://localhost:22000";
const client = {
  file: new FileServiceClient(url),
  profile: new ProfileServiceClient(url),
  recommendation: new RecommendationServiceClient(url),
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
