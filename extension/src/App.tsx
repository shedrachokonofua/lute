import { useEffect } from "react";
import { getPageContextMessage } from "./messages";
import { useCurrentTab } from "./hooks/use-current-tab";
import { DeferFn, useAsync } from "react-async";
import { PageType } from "./proto/lute_pb";
import { pageTypeToString } from "./helpers";
import { AppContextValue } from "./types";
import { getAllProfiles } from "./core";
import { AlbumAssessmentForm } from "./AlbumAssessmentForm";
import { SimilarAlbums } from "./SimilarAlbums";

const getAppContextValue: DeferFn<AppContextValue> = async ([url]) => {
  const [page, profiles] = await Promise.all([
    getPageContextMessage.send({ url }),
    getAllProfiles(),
  ]);
  return {
    page,
    profiles,
  };
};

export const App = () => {
  const currentTab = useCurrentTab();
  const {
    error: appContextValueError,
    isLoading: isAppContextValueLoading,
    data: appContextValue,
    run: loadAppContextValue,
  } = useAsync({
    deferFn: getAppContextValue,
  });
  useEffect(() => {
    if (currentTab) {
      loadAppContextValue(currentTab.url);
    }
  }, [currentTab, loadAppContextValue]);

  if (
    isAppContextValueLoading ||
    appContextValue === undefined ||
    appContextValue === null
  ) {
    return <p>Loading...</p>;
  }

  if (appContextValueError) {
    return <p>Error: {appContextValueError.message}</p>;
  }

  if (appContextValue.page === undefined) {
    return <p>Unsupported page type.</p>;
  }

  if (appContextValue.page?.pageType !== PageType.ALBUMPAGE) {
    return (
      <p>
        Recommendations not supported for page type:{" "}
        {pageTypeToString(appContextValue.page.pageType)}
      </p>
    );
  }

  return (
    <div
      style={{
        width: 240,
      }}
    >
      <div>
        <b>Lute</b>
      </div>
      <hr />
      <AlbumAssessmentForm context={appContextValue} />
      <hr />
      <SimilarAlbums context={appContextValue} />
    </div>
  );
};
