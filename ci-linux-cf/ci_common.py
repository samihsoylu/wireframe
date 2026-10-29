"""Small local support for the retained Linux child supervisor."""
import hashlib
import json
import os
from pathlib import Path


class Refused(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise Refused(message)


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode() + b'\n'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


class Journal:
    def __init__(self, path):
        self.path = Path(path)

    def record(self, event, **fields):
        data = encoded({'event': event, **fields})
        require(len(data) <= 32768, 'receipt row bound')
        require(not self.path.exists() or self.path.stat().st_size < 4 * 1024**2,
                'receipt journal bound')
        with self.path.open('ab') as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
