"""BACKWARD COMPATIBILITY MODULE ONLY"""


class RequestException(IOError):
    pass


class ConnectionError(RequestException):
    pass


class HTTPError(RequestException):
    pass


class Timeout(RequestException):
    pass


class ConnectTimeout(ConnectionError, Timeout):
    pass


class ReadTimeout(Timeout):
    pass


class SSLError(ConnectionError):
    pass


class ContentDecodingError(RequestException):
    pass
