# WireMock standalone container for performance testing
#
# Build: cd <repo-root> && tests/performance/wiremock/build.sh
# Run: docker run -p 8080:8080 -v /tmp/wiremock_mappings:/wiremock wiremock-perf:latest

FROM eclipse-temurin:17-jdk

RUN mkdir -p /opt/wiremock

WORKDIR /wiremock

COPY tests/wiremock/wiremock_standalone/wiremock-standalone-3.13.2.jar \
    /opt/wiremock/wiremock-standalone.jar

# Copy and compile custom launcher and extension
COPY tests/performance/wiremock/WireMockLauncher.java /opt/wiremock/
COPY tests/performance/wiremock/extensions/ResponseTimeExtension.java /opt/wiremock/extensions/
RUN javac -cp /opt/wiremock/wiremock-standalone.jar \
    /opt/wiremock/WireMockLauncher.java \
    /opt/wiremock/extensions/ResponseTimeExtension.java

# Generate static CA certificate for HTTPS MITM proxy
COPY tests/performance/wiremock/generate_certs.sh /opt/wiremock/
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
