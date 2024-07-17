import asyncio
import logging
import re
import sys
from time import time

from graphdatascience import GraphDataScience
from pythonjsonlogger import jsonlogger
from unidecode import unidecode

from graph.lute import LuteClient
from graph.proto import lute_pb2
from graph.settings import NEO4J_URL

logger = logging.getLogger()
logger.setLevel(logging.INFO)

logHandler = logging.StreamHandler(sys.stdout)
formatter = jsonlogger.JsonFormatter()
logHandler.setFormatter(formatter)
logger.addHandler(logHandler)


def is_album_parsed_event(item: lute_pb2.EventStreamItem) -> bool:
    return (
        item.HasField("payload")
        and item.payload.HasField("event")
        and item.payload.event.HasField("file_parsed")
        and item.payload.event.file_parsed.HasField("data")
        and item.payload.event.file_parsed.data.HasField("album")
    )


def setup_indexes(gds: GraphDataScience):
    statements = [
        """
        CREATE CONSTRAINT album_file_name IF NOT EXISTS FOR (a:Album)
        REQUIRE a.file_name IS UNIQUE
        """,
        """
        CREATE CONSTRAINT artist_file_name IF NOT EXISTS FOR (a:Artist)
        REQUIRE a.file_name IS UNIQUE
        """,
        """
        CREATE CONSTRAINT genre_name IF NOT EXISTS FOR (g:Genre)
        REQUIRE g.name IS UNIQUE
        """,
        """
        CREATE CONSTRAINT descriptor_name IF NOT EXISTS FOR (d:Descriptor)
        REQUIRE d.name IS UNIQUE
        """,
        """
        CREATE CONSTRAINT language_name IF NOT EXISTS FOR (l:Language)
        REQUIRE l.name IS UNIQUE
        """,
        "CREATE INDEX album_name IF NOT EXISTS FOR (a:Album) ON (a.name)",
        "CREATE INDEX artist_name IF NOT EXISTS FOR (a:Artist) ON (a.name)",
        "CREATE INDEX credited_role IF NOT EXISTS FOR ()-[r:CREDITED]-() ON (r.role)",
        "CREATE INDEX genre_weight IF NOT EXISTS FOR ()-[r:GENRE]-() ON (r.weight)",
    ]

    for statement in statements:
        gds.run_cypher(statement)


def cypher_var_name(group: str, name: str) -> str:
    name = unidecode(name).replace("-", "__")
    name = re.sub(r"[^a-zA-Z0-9]", "_", name)
    return f"{group}_{name}".lower()


def cypher_text(text: str) -> str:
    return unidecode(text).replace('"', '\\"')


def update_graph(gds: GraphDataScience, albums: list[tuple[str, lute_pb2.ParsedAlbum]]):
    start = time()
    relationship_count = 0

    logger.info("Building graph update", extra={"album_count": len(albums)})
    artists = {}
    genres = set()
    descriptors = set()
    language = set()

    for _, album in albums:
        for artist in album.artists:
            artists[artist.file_name] = artist.name
        for credit in album.credits:
            if credit.artist.file_name not in artists:
                artists[credit.artist.file_name] = credit.artist.name
        for genre in album.primary_genres:
            genres.add(genre)
        for genre in album.secondary_genres:
            genres.add(genre)
        for descriptor in album.descriptors:
            descriptors.add(descriptor)
        for lang in album.languages:
            language.add(lang)

        relationship_count += (
            len(album.artists)
            + len(album.credits)
            + len(album.primary_genres)
            + len(album.secondary_genres)
            + len(album.descriptors)
            + len(album.languages)
        )

    gds.run_cypher(
        """
        UNWIND $artists AS artist
        MERGE (a:Artist {file_name: artist.file_name})
        ON CREATE SET a.name = artist.name           
        """,
        {
            "artists": [
                {"file_name": file_name, "name": name}
                for file_name, name in artists.items()
            ]
        },
    )

    gds.run_cypher(
        """
        UNWIND $genres AS genre
        MERGE (g:Genre {name: genre})
        """,
        {"genres": list(genres)},
    )

    gds.run_cypher(
        """
        UNWIND $descriptors AS descriptor
        MERGE (d:Descriptor {name: descriptor})
        """,
        {"descriptors": list(descriptors)},
    )

    gds.run_cypher(
        """
        UNWIND $languages AS lang
        MERGE (l:Language {name: lang})
        """,
        {"languages": list(language)},
    )

    gds.run_cypher(
        """
        UNWIND $albums AS album
        MERGE (a:Album {file_name: album.file_name})
        ON CREATE SET a.name = album.name
        """,
        {
            "albums": [
                {"file_name": file_name, "name": album.name}
                for file_name, album in albums
            ]
        },
    )

    gds.run_cypher(
        """
        UNWIND $album_artists AS album_artist
        MATCH (album:Album {file_name: album_artist.album_file_name})
        MATCH (artist:Artist {file_name: album_artist.artist_file_name})
        MERGE (artist)-[:ALBUM_ARTIST]->(album)
        """,
        {
            "album_artists": [
                {"album_file_name": file_name, "artist_file_name": artist.file_name}
                for file_name, album in albums
                for artist in album.artists
            ]
        },
    )

    gds.run_cypher(
        """
        UNWIND $album_credits AS album_credit
        MATCH (album:Album {file_name: album_credit.album_file_name})
        MATCH (artist:Artist {file_name: album_credit.artist_file_name})
        MERGE (artist)-[:CREDITED {role: album_credit.role}]->(album)
        """,
        {
            "album_credits": [
                {
                    "album_file_name": file_name,
                    "artist_file_name": credit.artist.file_name,
                    "role": role,
                }
                for file_name, album in albums
                for credit in album.credits
                for role in credit.roles
            ]
        },
    )

    gds.run_cypher(
        """
        UNWIND $album_genres AS album_genre
        MATCH (album:Album {file_name: album_genre.album_file_name})
        MATCH (genre:Genre {name: album_genre.genre})
        MERGE (album)-[:GENRE {weight: 2}]->(genre)
        """,
        {
            "album_genres": [
                {"album_file_name": file_name, "genre": genre}
                for file_name, album in albums
                for genre in album.primary_genres
            ]
        },
    )

    gds.run_cypher(
        """
        UNWIND $album_genres AS album_genre
        MATCH (album:Album {file_name: album_genre.album_file_name})
        MATCH (genre:Genre {name: album_genre.genre})
        MERGE (album)-[:GENRE {weight: 1}]->(genre)
        """,
        {
            "album_genres": [
                {"album_file_name": file_name, "genre": genre}
                for file_name, album in albums
                for genre in album.secondary_genres
            ]
        },
    )

    gds.run_cypher(
        """
        UNWIND $album_descriptors AS album_descriptor
        MATCH (album:Album {file_name: album_descriptor.album_file_name})
        MATCH (descriptor:Descriptor {name: album_descriptor.descriptor})
        MERGE (album)-[:DESCRIPTOR]->(descriptor)
        """,
        {
            "album_descriptors": [
                {"album_file_name": file_name, "descriptor": descriptor}
                for file_name, album in albums
                for descriptor in album.descriptors
            ]
        },
    )

    gds.run_cypher(
        """
        UNWIND $album_languages AS album_language
        MATCH (album:Album {file_name: album_language.album_file_name})
        MATCH (lang:Language {name: album_language.lang})
        MERGE (album)-[:LANGUAGE]->(lang)
        """,
        {
            "album_languages": [
                {"album_file_name": file_name, "lang": lang}
                for file_name, album in albums
                for lang in album.languages
            ]
        },
    )

    node_count = (
        len(artists) + len(genres) + len(descriptors) + len(language) + len(albums)
    )
    logger.info(
        "Graph updated",
        extra={
            "album_count": len(albums),
            "duration": time() - start,
            "node_count": node_count,
            "relationship_count": relationship_count,
        },
    )


async def run():
    gds = GraphDataScience(NEO4J_URL)
    setup_indexes(gds)
    async with LuteClient() as client:
        async for items in client.stream_events("parser", "build", 500):
            logger.info("Received events", extra={"event_count": len(items)})
            parsed_albums = [
                (
                    item.payload.event.file_parsed.file_name,
                    item.payload.event.file_parsed.data.album,
                )
                for item in items
                if is_album_parsed_event(item)
            ]

            if parsed_albums:
                update_graph(gds, parsed_albums)
    gds.close()


def main():
    asyncio.run(run())
