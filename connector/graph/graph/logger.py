import logging
import sys

import json_logging

logger = logging.getLogger()
json_logging.config_root_logger()
logger.setLevel(logging.INFO)
logger.addHandler(logging.StreamHandler(sys.stdout))
