import { getTabContent, onTabLoaded } from "./browser";

const isRymTab = (tab: chrome.tabs.Tab) => {
  return tab.url?.startsWith("https://rateyourmusic.com/");
};

const getFileName = (url) =>
  new URL(url).pathname
    .split("/")
    .filter((x) => x !== "")
    .join("/");

export const onRymPageLoaded = (
  callback: (fileName: string, content: string) => void
) => {
  onTabLoaded(async (tab) => {
    if (!tab.url || !isRymTab(tab)) return;
    const fileName = getFileName(tab.url);
    const content = await getTabContent(tab);

    callback(fileName, content);
  });
};
