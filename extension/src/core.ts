import { FileServiceClient } from "./proto/LuteServiceClientPb";
import { GetFilePageTypeRequest, PutFileRequest } from "./proto/lute_pb";

const fileService = new FileServiceClient("http://localhost:22000");

export const putFile = async (fileName: string, content: string) => {
  const putFileRequest = new PutFileRequest();
  putFileRequest.setName(fileName);
  putFileRequest.setContent(content);

  await fileService.putFile(putFileRequest, {});
};

export const getFilePageType = async (fileName: string) => {
  const getFilePageTypeRequest = new GetFilePageTypeRequest();
  getFilePageTypeRequest.setName(fileName);

  const reply = await fileService.getFilePageType(getFilePageTypeRequest, {});
  return reply.getPageType();
};
