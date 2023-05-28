import "./polyfill";
import { isFileStale, putFile } from "./core";
import { onRymPageLoaded } from "./rym";

(async () => {
  onRymPageLoaded(async (fileName, content) => {
    try {
      if (!(await isFileStale(fileName))) {
        console.log(`File ${fileName} is not stale`);
        return;
      }
    } catch (e) {
      console.error(`Error checking if file ${fileName} is stale`, e);
      return;
    }

    await putFile(fileName, content);
  });
})();
