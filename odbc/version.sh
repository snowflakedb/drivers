BASE_VERSION=0.0.1
COMMIT_HASH="${COMMIT_HASH:-$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")}"
export VERSION="${BASE_VERSION}-${COMMIT_HASH}"
