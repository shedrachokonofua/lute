import { LuteClient } from "./proto/LuteServiceClientPb";
import { PutFileRequest, ValidateFileNameRequest } from "./proto/lute_pb";

const core = new LuteClient("http://localhost:22000");

export const validateFileName = async (fileName: string) =>
  (
    await core.validateFileName(
      new ValidateFileNameRequest().setName(fileName),
      {}
    )
  ).getValid();

export const isFileStale = async (fileName: string) =>
  (
    await core.isFileStale(new ValidateFileNameRequest().setName(fileName), {})
  ).getStale();

export const putFile = async (fileName: string, content: string) => {
  const putFileRequest = new PutFileRequest();
  putFileRequest.setName(fileName);
  putFileRequest.setContent(content);

  await core.putFile(putFileRequest, {});
};
