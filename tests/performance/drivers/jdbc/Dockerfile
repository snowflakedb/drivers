# Build with: cd tests/performance && hatch run build-jdbc
#
# The jar stage extends sf-core-builder (not a plain JDK image) because gradlew shadowJar runs
# generateProtobuf, which shells out to the Rust proto_generator in the workspace. The builder
# also carries the prebuilt libjdbc_bridge.so.
ARG BUILDPLATFORM=linux/amd64

# ============================================================================
# Stage: assemble the universal JDBC fat jar + compile the perf app
# ============================================================================
FROM --platform=${BUILDPLATFORM} sf-core-builder:latest AS jar_builder

# shadowJar needs JDK 11+; the driver still targets bytecode 1.8. sf-core-builder is Debian
# trixie, whose default JDK is 21 (matches the CI assembly JDK and the JRE 21 runtime below).
RUN apt-get update && apt-get install -y openjdk-21-jdk-headless \
    && rm -rf /var/lib/apt/lists/*

# JDBC project must sit in the workspace root so generateProtobuf resolves ../Cargo.toml.
COPY jdbc/ /workdir/jdbc/

# jdbc's settings.gradle + build.gradle resolve through Snowflake's Artifactory Maven proxy (instead
# of public Maven Central / the Gradle Plugin Portal, which rate-limit CI) only when JENKINS_HOME is
# set. A Docker build does not inherit the agent's env, so propagate it explicitly (build.sh passes
# the agent's JENKINS_HOME; empty on local dev, where the public repos are fine).
ARG JENKINS_HOME=
ENV JENKINS_HOME=${JENKINS_HOME}

RUN cd /workdir/jdbc && \
    ./gradlew --no-daemon --console=plain shadowJar && \
    cp build/libs/snowflake-jdbc-native-all-*.jar /workdir/snowflake-jdbc-native-all.jar

# Compile the perf app against the fat jar (which also supplies the shaded Jackson it uses).
COPY tests/performance/drivers/jdbc/app/ /workdir/app/
RUN mkdir -p /workdir/app-classes && \
    javac -cp "/workdir/snowflake-jdbc-native-all.jar" \
      -d /workdir/app-classes \
      $(find /workdir/app/src -name '*.java')

# Capture the Rust compiler version that built libjdbc_bridge.so (for run metadata).
RUN rustc --version | awk '{print $2}' | cut -d. -f1,2 > /workdir/rust_version

# curl + unzip for fetching the old jar and lifting the shaded Jackson out of the fat jar. Kept in
# its own layer after shadowJar so it does not invalidate the (expensive) fat-jar build cache.
RUN apt-get update && apt-get install -y curl unzip && rm -rf /var/lib/apt/lists/*

# The `old` comparison driver is the latest published snowflake-jdbc release — a self-contained fat
# jar that registers jdbc:snowflake:// via net.snowflake.client.jdbc.SnowflakeDriver, so the same
# compiled perf-app classes run against it unchanged. It bundles its own Jackson, but relocated under
# a different internal package, so the exact shaded package the app compiled against
# (net.snowflake.client.jdbc.internal.com.fasterxml.jackson) must be supplied separately on the `old`
# classpath — we lift the byte-identical classes out of our fat jar below.
# Fetched from Snowflake's internal Artifactory Maven proxy (not public Maven Central) to avoid a
# public CI dependency and its rate limits; the version is resolved from Maven metadata at build time.
RUN REPO=https://artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/artifactory/development-maven-virtual/net/snowflake/snowflake-jdbc && \
    VERSION=$(curl -fsSL "${REPO}/maven-metadata.xml" | grep -oP '<release>\K[^<]+') && \
    echo "Using snowflake-jdbc ${VERSION}" && \
    curl -fsSL -o /workdir/snowflake-jdbc-old.jar "${REPO}/${VERSION}/snowflake-jdbc-${VERSION}.jar"

RUN mkdir -p /workdir/shaded-jackson && cd /workdir/shaded-jackson && \
    unzip -q /workdir/snowflake-jdbc-native-all.jar \
      'net/snowflake/client/jdbc/internal/com/fasterxml/jackson/*' && \
    jar --create --file /workdir/shaded-jackson.jar -C /workdir/shaded-jackson .

# ============================================================================
# Base runtime image (shared by driver targets)
# ============================================================================
FROM --platform=${BUILDPLATFORM} eclipse-temurin:21-jre AS base

WORKDIR /opt/jdbc

RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

COPY --from=jar_builder /workdir/snowflake-jdbc-native-all.jar /opt/jdbc/snowflake-jdbc-native-all.jar
COPY --from=jar_builder /workdir/app-classes/ /opt/jdbc/app/

# Arrow requires nio access and native access on modern JDKs (mirrors the JDBC test task).
ENV JDBC_JAVA_OPTS="--add-opens=java.base/java.nio=ALL-UNNAMED --enable-native-access=ALL-UNNAMED"
ENV JDBC_CLASSPATH="/opt/jdbc/app:/opt/jdbc/snowflake-jdbc-native-all.jar"

# ============================================================================
# Universal driver image
# ============================================================================
FROM base AS universal

COPY --from=jar_builder /workdir/libjdbc_bridge.so /opt/jdbc/libjdbc_bridge.so
COPY --from=jar_builder /workdir/rust_version /etc/rust_version

ENV DRIVER_TYPE=universal
# NativeLibraryLoader resolves the JNI transport from CORE_PATH.
ENV CORE_PATH=/opt/jdbc/libjdbc_bridge.so

CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && \
    if [ -n "$WIREMOCK_CA_CERT" ] && [ -f "$WIREMOCK_CA_CERT" ]; then \
        CA_BUNDLE=/etc/ssl/certs/ca-certificates.crt; \
        printf '\n' >> "$CA_BUNDLE" && cat "$WIREMOCK_CA_CERT" >> "$CA_BUNDLE"; \
    fi && \
    java ${JDBC_JAVA_OPTS} -cp "${JDBC_CLASSPATH}" net.snowflake.perf.Main

# ============================================================================
# Old (comparison) driver image — released snowflake-jdbc, no libjdbc_bridge.
# ============================================================================
FROM base AS old

COPY --from=jar_builder /workdir/snowflake-jdbc-old.jar /opt/jdbc/snowflake-jdbc-old.jar
COPY --from=jar_builder /workdir/shaded-jackson.jar /opt/jdbc/shaded-jackson.jar
RUN echo "N/A" > /etc/rust_version

ENV DRIVER_TYPE=old
# The released jar relocates Jackson under a different package; supply the exact shaded package the
# app compiled against alongside it.
ENV JDBC_CLASSPATH="/opt/jdbc/app:/opt/jdbc/snowflake-jdbc-old.jar:/opt/jdbc/shaded-jackson.jar"

# No CORE_PATH: the released jar carries its own JNI transport. Results.writeRunMetadata already
# gates build_rust_version on driver_type=universal, so "N/A" here is never emitted to metadata.
# Old snowflake-jdbc's SFTrustManager uses the JVM default truststore (cacerts), not the OS CA
# bundle. Import the WireMock MITM CA there so replay does not fail TLS after useProxy is set.
CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && \
    if [ -n "$WIREMOCK_CA_CERT" ] && [ -f "$WIREMOCK_CA_CERT" ]; then \
        CA_BUNDLE=/etc/ssl/certs/ca-certificates.crt; \
        printf '\n' >> "$CA_BUNDLE" && cat "$WIREMOCK_CA_CERT" >> "$CA_BUNDLE"; \
        keytool -delete -alias wiremock -keystore "$JAVA_HOME/lib/security/cacerts" \
          -storepass changeit >/dev/null 2>&1 || true; \
        keytool -importcert -noprompt -alias wiremock \
          -file "$WIREMOCK_CA_CERT" \
          -keystore "$JAVA_HOME/lib/security/cacerts" \
          -storepass changeit; \
        echo "Old driver: imported WireMock CA into JVM cacerts"; \
    fi && \
    java ${JDBC_JAVA_OPTS} -cp "${JDBC_CLASSPATH}" net.snowflake.perf.Main
