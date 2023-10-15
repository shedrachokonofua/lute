import { useState, useEffect } from "react";

export function useCurrentTab() {
  const [currentTab, setCurrentTab] = useState<chrome.tabs.Tab>();

  useEffect(() => {
    chrome.tabs.query({ active: true, currentWindow: true }, function (tabs) {
      const [tab] = tabs;
      setCurrentTab(tab);
    });

    const handleTabUpdated = (
      _: number,
      __: chrome.tabs.TabChangeInfo,
      tab: chrome.tabs.Tab
    ) => {
      if (tab.active && tab.windowId === chrome.windows.WINDOW_ID_CURRENT) {
        setCurrentTab(tab);
      }
    };

    const handleTabActivated = (activeInfo: chrome.tabs.TabActiveInfo) => {
      chrome.tabs.get(activeInfo.tabId, function (tab) {
        setCurrentTab(tab);
      });
    };

    chrome.tabs.onUpdated.addListener(handleTabUpdated);
    chrome.tabs.onActivated.addListener(handleTabActivated);

    return () => {
      chrome.tabs.onUpdated.removeListener(handleTabUpdated);
      chrome.tabs.onActivated.removeListener(handleTabActivated);
    };
  }, []);

  return currentTab;
}
