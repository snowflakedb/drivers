# Lightweight HTTPS proxy for performance testing (recording and replay)
#
# ~200MB image, ~50MB runtime memory.
#
# Build: cd tests/performance/replay_server && docker build -t replay-server:latest .

FROM python:3.13-slim

RUN pip install --no-cache-dir cryptography

COPY server.py /opt/proxy/server.py

ENTRYPOINT ["python", "/opt/proxy/server.py"]
