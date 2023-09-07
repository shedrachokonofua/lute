# Lute

**Lute** is a [RateYourMusic](rateyourmusic.com) album scraper and recommendation engine. It is a power-tool for music discovery that allows you to curate a self-hosted music database and generate personalized recommendations.

## Features

- **Efficient Scraping**: Crawl and index albums from RateYourMusic.
- **Polite**: Fully configurable crawler concurrency, rate-limiting, backoff, max queue size, and data staleness checks. Remember we love RYM and don't want to get banned.
- **Personalized Collections**: Curate albums into "Profiles" for tailored recommendations.
- **Spotify Integration**: Import albums from your Spotify catalogue into profiles.
- **Advanced Recommendation Methods**:
  - Quantile Ranking
  - [Coming Soon] Vector Similarity Search: Using OpenAI's API for album embeddings.
- **Browser Extension**: Parse and index albums from RYM in real-time while browsing the site.
- **Proxy Support**: Bring your own crawler proxy for uninterrupted scraping.
- **Interfaces**: GRPC API and Web-based UI.
- **Data Export**: Export to Postgres and Bolt-compatible graph databases using connectors.
- **Monitoring**: OpenTelemetry support for diagnostics.

**Disclaimer**: This project is for educational purposes only. Excessive scraping of RateYourMusic may result in your IP being banned.
