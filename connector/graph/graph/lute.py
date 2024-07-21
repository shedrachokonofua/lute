import asyncio
from typing import AsyncIterator, Optional

from google.protobuf import empty_pb2
from grpc import aio

import graph.proto.lute_pb2 as lute_pb2
import graph.proto.lute_pb2_grpc as lute_pb2_grpc
from graph.models import EmbeddingDocument
from graph.settings import LUTE_EVENT_SUBSCRIBER_PREFIX, LUTE_URL

MAX_MESSAGE_LENGTH = 1024 * 1024 * 1024


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

    async def get_event_monitor(self) -> lute_pb2.EventsMonitor:
        if self.event_service is None:
            raise ValueError("Client not initialized")

        response = await self.event_service.GetMonitor(empty_pb2.Empty())
        return response.monitor

    async def get_subscriber_cursor(self, subscriber_id) -> Optional[str]:
        if self.event_service is None:
            raise ValueError("Client not initialized")

        subscriber_id = f"{LUTE_EVENT_SUBSCRIBER_PREFIX}:{subscriber_id}"
        monitor = await self.get_event_monitor()

        for subscriber in monitor.subscribers:
            if subscriber.id == subscriber_id:
                return subscriber.cursor

        return None

    async def stream_events(
        self, stream_id, subscriber_id, max_batch_size=250
    ) -> AsyncIterator[list[lute_pb2.EventStreamItem]]:
        if self.event_service is None:
            raise ValueError("Client not initialized")

        subscriber_id = f"{LUTE_EVENT_SUBSCRIBER_PREFIX}:{subscriber_id}"

        queue = asyncio.Queue()

        async def request_generator():
            yield lute_pb2.EventStreamRequest(
                stream_id=stream_id,
                subscriber_id=subscriber_id,
                max_batch_size=max_batch_size,
            )

            while True:
                cursor = await queue.get()
                yield lute_pb2.EventStreamRequest(
                    stream_id=stream_id,
                    subscriber_id=subscriber_id,
                    cursor=cursor,
                    max_batch_size=max_batch_size,
                )
                await asyncio.sleep(0.25)

        async for reply in self.event_service.Stream(request_generator()):
            yield reply.items
            await queue.put(reply.cursor)

    async def bulk_upload_embeddings(
        self, embedding_iter: AsyncIterator[list[EmbeddingDocument]]
    ) -> int:
        if self.album_service is None:
            raise ValueError("Client not initialized")

        async def request_generator():
            async for batch in embedding_iter:
                yield lute_pb2.BulkUploadAlbumEmbeddingsRequest(
                    items=[
                        lute_pb2.BulkUploadAlbumEmbeddingsRequestItem(
                            file_name=doc.file_name,
                            embedding=doc.embedding,
                            embedding_key=doc.embedding_key,
                        )
                        for doc in batch
                    ]
                )

        reply = await self.album_service.BulkUploadAlbumEmbeddings(request_generator())
        return reply.count
