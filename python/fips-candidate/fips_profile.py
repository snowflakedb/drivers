"""Private-key compatibility policy for the FIPS-TLS candidate distribution."""

ALLOW_CRYPTOGRAPHY_PRIVATE_KEY_OBJECTS = False

# Identifies the distribution expected to supply the native bridge, not a
# FIPS certification or validation claim.
FIPS_TLS_CANDIDATE = True
