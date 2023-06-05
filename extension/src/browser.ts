export const getTabContent = async (tab: chrome.tabs.Tab) => {
  const [{ result: content }] = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => document.documentElement.outerHTML,
  });

  return content;
};

export const onTabLoaded = (callback: (tab: chrome.tabs.Tab) => void) => {
  chrome.tabs.onUpdated.addListener(async (_, changeInfo, tab) => {
    if (changeInfo.status === "complete") {
      // Wait for the page to be fully loaded
      await new Promise((resolve) => setTimeout(resolve, 750));

      callback(tab);
    }
  });
};
