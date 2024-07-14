from grpc import aio
import graph.proto.lute_pb2 as lute_pb2
import graph.proto.lute_pb2_grpc as lute_pb2_grpc
from google.protobuf import empty_pb2
from typing import Optional
from graph.settings import LUTE_EVENT_SUBSCRIBER_PREFIX, LUTE_URL


class LuteClient:
    def __init__(self):
        self.channel: Optional[aio.Channel] = None
        self.album_service: Optional[lute_pb2_grpc.AlbumServiceStub] = None
        self.event_service: Optional[lute_pb2_grpc.EventServiceStub] = None

    async def __aenter__(self):
        self.channel = aio.insecure_channel(LUTE_URL)
        self.album_service = lute_pb2_grpc.AlbumServiceStub(self.channel)
        self.event_service = lute_pb2_grpc.EventServiceStub(self.channel)
        return self

    async def __aexit__(self, exc_type, exc_value, traceback):
        if self.channel:
            await self.channel.close()

    async def get_album(self, file_name):
        if self.album_service is None:
            raise ValueError("Client not initialized")

        reply = await self.album_service.GetAlbum(
            lute_pb2.GetAlbumRequest(file_name=file_name)
        )
        return reply.album

    async def stream_events(self, stream_id, subscriber_id):
        if self.event_service is None:
            raise ValueError("Client not initialized")

        subscriber_id = f"{LUTE_EVENT_SUBSCRIBER_PREFIX}:{subscriber_id}"

        # Help me complete
