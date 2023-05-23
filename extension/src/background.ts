import "./polyfill";
import { LuteClient } from "./proto/LuteServiceClientPb";
import { ValidateFileNameRequest } from "./proto/lute_pb";
import { PutFileRequest } from "./proto/lute_pb";

export const core = new LuteClient("http://localhost:22000");

const onTabLoaded = (callback: (tab: chrome.tabs.Tab) => void) => {
  chrome.tabs.onUpdated.addListener((_, changeInfo, tab) => {
    if (changeInfo.status === "complete") {
      callback(tab);
    }
  });
};

const isRymTab = (tab: chrome.tabs.Tab) => {
  return tab.url?.startsWith("https://rateyourmusic.com/");
};

const onRymPageLoaded = (callback: (tab: chrome.tabs.Tab) => void) => {
  onTabLoaded((tab) => {
    if (!isRymTab(tab)) return;
    callback(tab);
  });
};

const getFileName = (url) =>
  new URL(url).pathname
    .split("/")
    .filter((x) => x !== "")
    .join("/");

const isFileNameValid = async (fileName: string) =>
  (
    await core.validateFileName(
      new ValidateFileNameRequest().setName(fileName),
      {}
    )
  ).getValid();

const isFileStale = async (fileName: string) =>
  (
    await core.isFileStale(new ValidateFileNameRequest().setName(fileName), {})
  ).getStale();

const shouldUpload = async (tab: chrome.tabs.Tab) => {
  if (!tab.url) return false;
  const fileName = getFileName(tab.url);

  if (!(await isFileNameValid(fileName))) {
    console.log(`Invalid file name: ${fileName}`);
    return false;
  }

  const isStale = await isFileStale(fileName);

  console.log(`File ${fileName} is stale: ${isStale}`);

  return isStale;
};

const uploadTabContent = async (tab: chrome.tabs.Tab) => {
  if (!tab.id || !tab.url) return;

  const fileName = getFileName(tab.url);

  const [{ result: content }] = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => document.documentElement.outerHTML,
  });

  const putFileRequest = new PutFileRequest();
  putFileRequest.setName(fileName);
  putFileRequest.setContent(content);

  await core.putFile(putFileRequest, {});
};

(async () => {
  onRymPageLoaded(async (tab) => {
    if (!(await shouldUpload(tab))) return;
    await uploadTabContent(tab);
  });
})();
