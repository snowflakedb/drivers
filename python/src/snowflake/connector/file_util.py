"""BACKWARD COMPATIBILITY MODULE ONLY"""

from __future__ import annotations

from typing import IO


class SnowflakeFileUtil:
    @staticmethod
    def get_digest_and_size(src: IO[bytes]) -> tuple[str, int]:
        raise NotImplementedError("get_digest_and_size is not yet implemented")

    @staticmethod
    def compress_with_gzip_from_stream(src_stream: IO[bytes]) -> tuple[IO[bytes], int]:
        raise NotImplementedError("compress_with_gzip_from_stream is not yet implemented")

    @staticmethod
    def compress_file_with_gzip(file_name: str, tmp_dir: str) -> tuple[str, int]:
        raise NotImplementedError("compress_file_with_gzip is not yet implemented")
