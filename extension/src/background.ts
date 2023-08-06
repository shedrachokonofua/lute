import "./polyfill";
import { putFile } from "./core";
import { onRymPageLoaded } from "./rym";

(async () => {
  onRymPageLoaded(putFile);
})();
