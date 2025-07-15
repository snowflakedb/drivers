from thrift.protocol.TCompactProtocol import TCompactProtocol

from .c_api import CORE_API, sf_core_api_init
from .transport import CoreTransport
from ..thrift_gen.database_driver_v1 import DatabaseDriver

def client_api_init(api_id):
    if api_id == CORE_API.DATABASE_DRIVER_API_V1:
        channel = sf_core_api_init(api_id)
        transport = CoreTransport(channel)
        transport.open()
        protocol = TCompactProtocol(transport)
        return DatabaseDriver.Client(protocol)