import asyncio

from graphdatascience import GraphDataScience

from graph import api, db
from graph.logger import logger
from graph.lute import LuteClient
from graph.proto import lute_pb2
from graph.settings import NEO4J_URL


def is_album_parsed_event(item: lute_pb2.EventStreamItem) -> bool:
    return (
        item.HasField("payload")
        and item.payload.HasField("event")
        and item.payload.event.HasField("file_parsed")
        and item.payload.event.file_parsed.HasField("data")
        and item.payload.event.file_parsed.data.HasField("album")
    )


async def run_graph_sync():
    gds = GraphDataScience(NEO4J_URL)
    db.setup_indexes(gds)
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
                db.update_graph(gds, parsed_albums)
    gds.close()


async def run():
    await asyncio.gather(api.run(), run_graph_sync())


def main():
    asyncio.run(run())


# if __name__ == "__main__":
#     main()
