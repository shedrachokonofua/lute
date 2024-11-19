const { lute } = require("./proto/lute");
const { enqueueCrawl, getAllArtists } = require("./shared/lute");

(async () => {
  const artists = await getAllArtists();
  console.log(`Found ${artists.length} artists`);
  for (const artist of artists) {
    await enqueueCrawl(artist.artist.fileName, lute.Priority.Low);
  }
})();
