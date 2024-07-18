import os

API_PORT = int(os.environ.get("API_PORT", 22005))
LUTE_URL = os.environ.get("LUTE_URL", "localhost:22000")
NEO4J_URL = os.environ.get("NEO4J_URL", "bolt://localhost:7687")
LUTE_EVENT_SUBSCRIBER_PREFIX = os.environ.get(
    "LUTE_EVENT_SUBSCRIBER_PREFIX", "graph-connector"
)
