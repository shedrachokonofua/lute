from time import time
from graph.lute import LuteClient
import asyncio
from graph.proto import lute_pb2
from graphdatascience import GraphDataScience
from graph.settings import NEO4J_URL
import re
from unidecode import unidecode
import logging
import sys
from pythonjsonlogger import jsonlogger

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
        "CREATE CONSTRAINT album_file_name IF NOT EXISTS FOR (a:Album) REQUIRE a.file_name IS UNIQUE",
        "CREATE CONSTRAINT artist_file_name IF NOT EXISTS FOR (a:Artist) REQUIRE a.file_name IS UNIQUE",
        "CREATE CONSTRAINT genre_name IF NOT EXISTS FOR (g:Genre) REQUIRE g.name IS UNIQUE",
        "CREATE CONSTRAINT descriptor_name IF NOT EXISTS FOR (d:Descriptor) REQUIRE d.name IS UNIQUE",
        "CREATE CONSTRAINT language_name IF NOT EXISTS FOR (l:Language) REQUIRE l.name IS UNIQUE",
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

    cypher = ""

    for genre in genres:
        name = cypher_var_name("genre", genre)
        cypher += f"""
        MERGE ({name}:Genre {{name: "{cypher_text(genre)}"}})
        """

    for descriptor in descriptors:
        name = cypher_var_name("descriptor", descriptor)
        cypher += f"""
        MERGE ({name}:Descriptor {{name: "{descriptor}"}})
        """

    for lang in language:
        name = cypher_var_name("lang", lang)
        cypher += f"""
        MERGE ({name}:Language {{name: "{lang}"}})
        """

    for file_name, name in artists.items():
        c_name = cypher_var_name("artist", file_name)
        cypher += f"""
        MERGE ({c_name}:Artist {{file_name: "{file_name}"}})
        ON CREATE SET {c_name}.name = "{cypher_text(name)}"
        """

    for file_name, album in albums:
        name = cypher_var_name("album", file_name)
        cypher += f"""
        MERGE ({name}:Album {{file_name: "{file_name}"}})
        ON CREATE SET {name}.name = "{cypher_text(album.name)}"
        """

        for artist in album.artists:
            artist_name = cypher_var_name("artist", artist.file_name)
            cypher += f"MERGE ({artist_name})-[:ALBUM_ARTIST]->({name})"

        for credit in album.credits:
            artist_name = cypher_var_name("artist", credit.artist.file_name)
            for role in credit.roles:
                role_name = cypher_var_name("role", role)
                cypher += f"MERGE ({artist_name})-[:CREDITED {{role: '{role_name}'}}]->({name})"

        for genre in album.primary_genres:
            genre_name = cypher_var_name("genre", genre)
            cypher += f"MERGE ({name})-[:GENRE {{weight: 2}}]->({genre_name})"

        for genre in album.secondary_genres:
            genre_name = cypher_var_name("genre", genre)
            cypher += f"MERGE ({name})-[:GENRE {{weight: 1}}]->({genre_name})"

        for descriptor in album.descriptors:
            descriptor_name = cypher_var_name("descriptor", descriptor)
            cypher += f"MERGE ({name})-[:DESCRIPTOR]->({descriptor_name})"

        for lang in album.languages:
            lang_name = cypher_var_name("lang", lang)
            cypher += f"MERGE ({name})-[:LANGUAGE]->({lang_name})"

    gds.run_cypher(cypher)
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
        async for items in client.stream_events("parser", "build", 10):
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
