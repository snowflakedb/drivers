"""Private-key compatibility policy selected by the installed distribution."""

ALLOW_CRYPTOGRAPHY_PRIVATE_KEY_OBJECTS = True

# Identifies the distribution expected to supply the native bridge, not a
# FIPS certification or validation claim.
FIPS_TLS_CANDIDATE = False
