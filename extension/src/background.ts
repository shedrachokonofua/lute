import "./polyfill";
import { putFile, getFilePageType } from "./core";

const isRymTab = (tab: chrome.tabs.Tab) => {
  return tab.url?.startsWith("https://rateyourmusic.com/");
};

const getFileName = (tab: chrome.tabs.Tab) => {
  const base = decodeURI(
    new URL(tab.url).pathname
      .split("/")
      .filter((x) => x !== "")
      .join("/")
  );
  const queryPart = base.startsWith("search") ? new URL(tab.url).search : "";
  return base + queryPart;
};

export const getTabContent = async (tab: chrome.tabs.Tab) => {
  const [{ result: content }] = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => document.documentElement.outerHTML,
  });

  return content;
};

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const pageTypeCache = new Map<string, Number>();

const loadFilePageType = async (fileName: string) => {
  if (pageTypeCache.has(fileName)) return pageTypeCache.get(fileName);
  const pageType = await getFilePageType(fileName);
  pageTypeCache.set(fileName, pageType);
  console.log(`Loaded page type for ${fileName}: ${pageType}`);
  return pageType;
};

(async () => {
  chrome.tabs.onUpdated.addListener(async (_, changeInfo, tab) => {
    if (!tab.url || !isRymTab(tab) || changeInfo.status !== "complete") return;
    const fileName = getFileName(tab);
    try {
      await Promise.all([
        loadFilePageType(fileName),
        delay(750), // wait for the page to be fully loaded
      ]);
    } catch (e) {
      console.log(`Unsupported page: ${fileName}`);
      return;
    }
    const content = await getTabContent(tab);
    await putFile(fileName, content);
  });
})();
