import "./polyfill";
import { putFile, getFilePageType, deleteFile } from "./core";
import { PageType } from "./proto/lute_pb";
import { getPageContextMessage } from "./messages";
import { getFileName } from "./helpers";

const isRym = (url: string) => url.startsWith("https://rateyourmusic.com/");

export const getTabContent = async (tab: chrome.tabs.Tab) => {
  if (!tab?.id) throw new Error("Tab has no ID");
  const [{ result: content }] = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => document.documentElement.outerHTML,
  });

  return content;
};

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const unsupportedPageCache = new Set<string>();
const pageTypeCache = new Map<string, Promise<PageType>>();

const loadFilePageType = async (fileName: string) => {
  if (pageTypeCache.has(fileName)) return pageTypeCache.get(fileName);
  const pageType = getFilePageType(fileName);
  pageTypeCache.set(fileName, pageType);
  return pageType;
};

(async () => {
  chrome.tabs.onUpdated.addListener(async (_, changeInfo, tab) => {
    if (!tab.url || !isRym(tab.url) || changeInfo.status !== "complete") return;

    const fileName = getFileName(tab.url);
    if (unsupportedPageCache.has(fileName)) {
      console.log(`Cached unsupported page: ${fileName}`);
      return;
    }

    try {
      await Promise.all([
        loadFilePageType(fileName),
        delay(750), // wait for the page to be fully loaded
      ]);
    } catch (e) {
      console.log(`Unsupported page: ${fileName}`);
      unsupportedPageCache.add(fileName);
      return;
    }

    const content = await getTabContent(tab);
    await putFile(fileName, content);
  });

  getPageContextMessage.listen(async ({ url }) => {
    if (!isRym(url)) return undefined;
    const fileName = getFileName(url);
    if (unsupportedPageCache.has(fileName)) return undefined;
    try {
      const pageType = await loadFilePageType(fileName);
      if (pageType === undefined) return undefined;
      return {
        fileName,
        pageType,
      };
    } catch (e) {
      return undefined;
    }
  });

  chrome.webRequest.onBeforeRedirect.addListener(
    async (details) => {
      if (!isRym(details.url) || !isRym(details.redirectUrl)) return;
      const oldFileName = getFileName(details.url);
      const newFileName = getFileName(details.redirectUrl);
      if (oldFileName === newFileName) return;

      console.log("Deleting moved file: ", oldFileName);
      try {
        unsupportedPageCache.delete(oldFileName);
        await deleteFile(oldFileName);
      } catch (e) {
        console.log({
          message: "Failed to delete moved file",
          oldFileName,
          newFileName,
          e,
        });
      }
    },
    {
      urls: ["<all_urls>"],
    }
  );
})();
