# WireMock standalone container for performance testing
# 
# Build: cd tests/performance/wiremock && docker build -t wiremock-perf:latest .
# Run: docker run -p 8080:8080 -v /tmp/wiremock_mappings:/wiremock wiremock-perf:latest

FROM eclipse-temurin:17-jdk

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create directory for WireMock
RUN mkdir -p /opt/wiremock

WORKDIR /wiremock

# Download WireMock standalone
ARG WIREMOCK_VERSION=3.13.2
RUN curl -L "https://repo1.maven.org/maven2/org/wiremock/wiremock-standalone/${WIREMOCK_VERSION}/wiremock-standalone-${WIREMOCK_VERSION}.jar" \
    -o /opt/wiremock/wiremock-standalone.jar

# Copy and compile custom launcher and extension
COPY WireMockLauncher.java /opt/wiremock/
COPY extensions/ResponseTimeExtension.java /opt/wiremock/extensions/
RUN javac -cp /opt/wiremock/wiremock-standalone.jar \
    /opt/wiremock/WireMockLauncher.java \
    /opt/wiremock/extensions/ResponseTimeExtension.java

# Generate static CA certificate for HTTPS MITM proxy
COPY generate_certs.sh /opt/wiremock/
RUN /opt/wiremock/generate_certs.sh

# Expose WireMock port
EXPOSE 8080

# Create entrypoint script that respects JAVA_OPTS for heap configuration
RUN printf '#!/bin/bash\n\
set -e\n\
\n\
# Use JAVA_OPTS if set, otherwise use defaults\n\
if [ -z "$JAVA_OPTS" ]; then\n\
    JAVA_OPTS="-Xmx1g -Xms512m"\n\
fi\n\
\n\
echo "Started WireMock with JAVA_OPTS: $JAVA_OPTS"\n\
\n\
exec java $JAVA_OPTS -cp /opt/wiremock:/opt/wiremock/wiremock-standalone.jar WireMockLauncher "$@"\n\
' > /usr/local/bin/docker-entrypoint.sh && chmod +x /usr/local/bin/docker-entrypoint.sh

# Set entrypoint to wrapper script
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

# Default command (will be overridden by Python manager)
CMD ["--port", "8080"]
