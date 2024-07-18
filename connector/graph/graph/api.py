from contextlib import asynccontextmanager

import uvicorn
from fastapi import FastAPI

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


@app.get("/")
async def root():
    return {
        "lute_cursors": {
            "build": await lute_client.get_subscriber_cursor("build"),
        }
    }


async def run():
    config = uvicorn.Config(
        app, host="0.0.0.0", port=API_PORT, log_level="info", reload=True
    )
    server = uvicorn.Server(config)
    await server.serve()
