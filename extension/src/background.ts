import "./polyfill";
import { isFileStale, putFile, validateFileName } from "./core";
import { onRymPageLoaded } from "./rym";

(async () => {
  onRymPageLoaded(async (fileName, content) => {
    if (!(await validateFileName(fileName))) {
      console.log(`Invalid file name: ${fileName}`);
      return;
    }

    if (!(await isFileStale(fileName))) {
      console.log(`File ${fileName} is not stale`);
      return;
    }

    await putFile(fileName, content);
  });
})();
