export const getTabContent = async (tab: chrome.tabs.Tab) => {
  const [{ result: content }] = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => document.documentElement.outerHTML,
  });

  return content;
};

export const onTabLoaded = (callback: (tab: chrome.tabs.Tab) => void) => {
  chrome.tabs.onUpdated.addListener((_, changeInfo, tab) => {
    if (changeInfo.status === "complete") {
      callback(tab);
    }
  });
};
