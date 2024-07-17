import asyncio
from typing import AsyncIterator, Optional

from grpc import aio

import graph.proto.lute_pb2 as lute_pb2
import graph.proto.lute_pb2_grpc as lute_pb2_grpc
from graph.settings import LUTE_EVENT_SUBSCRIBER_PREFIX, LUTE_URL

MAX_MESSAGE_LENGTH = 50 * 1024 * 1024


class LuteClient:
    def __init__(self):
        self.channel: Optional[aio.Channel] = None
        self.album_service: Optional[lute_pb2_grpc.AlbumServiceStub] = None
        self.event_service: Optional[lute_pb2_grpc.EventServiceStub] = None

    async def __aenter__(self):
        self.channel = aio.insecure_channel(
            LUTE_URL,
            options=[
                ("grpc.max_send_message_length", MAX_MESSAGE_LENGTH),
                ("grpc.max_receive_message_length", MAX_MESSAGE_LENGTH),
            ],
        )
        self.album_service = lute_pb2_grpc.AlbumServiceStub(self.channel)
        self.event_service = lute_pb2_grpc.EventServiceStub(self.channel)
        return self

    async def __aexit__(self, exc_type, exc_value, traceback):
        if self.channel:
            await self.channel.close()

    async def stream_events(
        self, stream_id, subscriber_id, max_batch_size=250
    ) -> AsyncIterator[list[lute_pb2.EventStreamItem]]:
        if self.event_service is None:
            raise ValueError("Client not initialized")

        subscriber_id = f"{LUTE_EVENT_SUBSCRIBER_PREFIX}:{subscriber_id}"
        cursor = None

        async def request_generator():
            while True:
                yield lute_pb2.EventStreamRequest(
                    stream_id=stream_id,
                    subscriber_id=subscriber_id,
                    cursor=cursor,
                    max_batch_size=max_batch_size,
                )
                await asyncio.sleep(0.25)

        async for reply in self.event_service.Stream(request_generator()):
            cursor = reply.cursor
            yield reply.items
