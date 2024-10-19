const { lute } = require("./proto/lute");
const { enqueueCrawl } = require("./shared/lute");

(async () => {
  for (const type of ["album", "mixtape", "ep"]) {
    for (let year = 2024; year >= 1974; year--) {
      for (let page = 1; page <= 26; page++) {
        const fileName = `charts/top/${type}/${year}/${page}`;
        await enqueueCrawl(fileName, lute.Priority.Express);
      }
    }
  }
})();
