import "./polyfill";
import { putFile, getFilePageType } from "./core";
import { PageType } from "./proto/lute_pb";

const isRymTab = (tab: chrome.tabs.Tab) =>
  tab.url?.startsWith("https://rateyourmusic.com/");

const getFileName = (tab: chrome.tabs.Tab) => {
  if (!tab?.url) throw new Error("Tab has no URL");
  const url = new URL(tab.url);
  const base = decodeURI(
    url.pathname
      .split("/")
      .filter((x) => x !== "")
      .join("/")
  );
  const queryPart = base.startsWith("search") ? url.search : "";
  return base + queryPart;
};

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
    if (!tab.url || !isRymTab(tab) || changeInfo.status !== "complete") return;

    const fileName = getFileName(tab);
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
})();
