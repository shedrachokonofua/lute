from graph.lute import LuteClient
import asyncio


async def run():
    async with LuteClient() as client:
        monitor = await client.get_album("release/album/nas/illmatic")
        print(monitor)


def main():
    asyncio.run(run())
