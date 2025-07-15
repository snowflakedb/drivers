from thrift.transport.TTransport import TTransportBase
import ctypes

from pep249_dbapi.api_client.c_api import sf_core_api_read, sf_core_api_write, sf_core_api_flush


class CoreTransport(TTransportBase):
    """Base class for Thrift transport layer."""
    def __init__(self, channel):
        self.channel = channel

    def isOpen(self):
        return True

    def open(self):
        pass

    def close(self):
        pass

    def read(self, sz):
        py_buffer = bytearray(b'\0' * sz)
        c_buffer = (ctypes.c_char * sz).from_buffer(py_buffer)
        sf_core_api_read(self.channel, c_buffer, sz)
        return py_buffer


    def write(self, buf):
        py_buffer = bytearray(buf)
        c_buffer = (ctypes.c_char * len(buf)).from_buffer(py_buffer)
        c_len = ctypes.c_size_t(len(buf))
        sf_core_api_write(self.channel, c_buffer, c_len)

    def flush(self):
        sf_core_api_flush(self.channel)


