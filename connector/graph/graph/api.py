from contextlib import asynccontextmanager

import json_logging
import uvicorn
from fastapi import FastAPI

from graph import db
from graph.logger import logger
from graph.lute import LuteClient
from graph.settings import API_PORT

lute_client = LuteClient()


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup: initialize the LuteClient
    await lute_client.__aenter__()
    yield
    # Shutdown: close the LuteClient
    await lute_client.__aexit__(None, None, None)


app = FastAPI(lifespan=lifespan)
json_logging.init_fastapi(enable_json=True)
json_logging.init_request_instrument(app)


@app.get("/")
async def root():
    return {
        "lute_cursors": {
            "build": await lute_client.get_subscriber_cursor("build"),
        }
    }


@app.post("/embeddings/albums/sync")
async def sync_album_embeddings(relationship_weights: db.AlbumRelationWeights):
    embeddings = db.generate_album_embeddings("lute_graph", relationship_weights)
    logger.info(
        "Generated embeddings", extra={"props": {"embedding_count": len(embeddings)}}
    )

    cursor = 0
    batch_size = 1500

    async def upload_generator():
        nonlocal cursor
        while cursor < len(embeddings):
            batch = embeddings[cursor : cursor + batch_size]
            logger.info(
                "Uploading embeddings batch",
                extra={"props": {"batch_size": len(batch), "cursor": cursor}},
            )
            yield batch
            cursor += batch_size

    node_count = await lute_client.bulk_upload_embeddings(upload_generator())

    return {"node_count": node_count}


async def run():
    config = uvicorn.Config(
        app,
        host="0.0.0.0",
        port=API_PORT,
        reload=True,
        log_config=None,
    )
    server = uvicorn.Server(config)
    await server.serve()
