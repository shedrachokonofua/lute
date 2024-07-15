# Lute [Work in Progress]

**Lute** is a [RateYourMusic](https://www.rateyourmusic.com) scraper and music recommendation engine. It is a power-tool for music discovery that allows you to curate a self-hosted music database and generate personalized recommendations.

## Features

- **Efficient Scraping**: Retrieve, parse and index albums, artists, charts, and lists from RateYourMusic.
- **Polite**: Fully configurable crawler concurrency, rate-limiting, backoff, max queue size, and data staleness checks. Remember we love RYM and don't want to get banned.
- **Personalized Collections**: Curate albums into "Profiles" for tailored recommendations.
- **Spotify Integration**: Import albums from your Spotify saved tracks or playlists into profiles.
- **Advanced Discovery and Recommendation Capabilities**:
  - Search and Discovery: Find albums and artists based on various criteria, including similarity to other albums or artists.
  - Quantile Ranking: A custom content-based filtering algorithm that uses weighted percentile ranking of feature frequencies across multiple categories (genres, descriptors, etc.) in a collection to efficiently rank albums for recommendations.
  - Vector Similarity: KNN-based recommendations using embeddings, with support for various providers (OpenAI, VoyageAI, Ollama).
- **Connectors**:
  - Data Export: Export to Postgres and Neo4j.
  - Discord Bot: AI chatbot for music recommendations.
  - Mandolin: [WIP] Lightweight app for recommendations based on RateYourMusic lists.
- **Proxy Support**: Bring your own crawler proxy for uninterrupted scraping.
- **API**: GRPC API for programmatic access.
- **Admin UI**: A user-friendly interface for managing your music database, and visualizing recommendation data.
- **Browser Extension**: Parse and index data from RYM in real-time while browsing the site.
- **Durable and Resilient Architecture**: Event stream-based design supporting state replay and work recovery.
- **Monitoring**: OpenTelemetry support for advanced diagnostics and tracing.

**Disclaimer**: This project is for educational and personal use only. Respect RateYourMusic's terms of service and use the tool responsibly.

## Deployment

## Development
