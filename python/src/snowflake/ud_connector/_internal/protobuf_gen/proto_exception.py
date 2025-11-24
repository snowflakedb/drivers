class ProtoException(Exception):
    pass

class ProtoApplicationException(ProtoException):
    def __init__(self, error, *args, **kwargs):
        self.api_error_pb = error
        super().__init__(error, *args, **kwargs)

    def __getattr__(self, name):
        try:
            return getattr(self.api_error_pb, name)
        except AttributeError:
            raise AttributeError(f"'{type(self).__name__}' underlying protobuf object has no attribute '{name}'")


class ProtoTransportException(ProtoException):
    pass