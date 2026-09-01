import { describe, it } from 'vitest';

// Parked for later: these are e2e tests that need utilities to exercise the TLS
// handshake against a wiremock / other mock server before they can assert real
// behavior. Kept as todos so the intended NODE_TLS_REJECT_UNAUTHORIZED /
// NODE_EXTRA_CA_CERTS coverage is tracked rather than forgotten.
describe('node TLS environment options', () => {
  it.todo('disables certificate verification when NODE_TLS_REJECT_UNAUTHORIZED=0');

  it.todo('loads a custom CA bundle when NODE_EXTRA_CA_CERTS points at a PEM file');

  // TODO: Node's NODE_EXTRA_CA_CERTS adds the file's certs on top of the built-in
  // root bundle, but the core's custom_root_store_path replaces the built-in roots
  // with only the supplied file (tls_built_in_root_certs(false) in
  // sf_core/src/tls/client.rs). Fix the core to make custom roots additive, then
  // cover it here: a server chaining to a built-in root still verifies while
  // NODE_EXTRA_CA_CERTS is set.
  it.todo('keeps built-in roots trusted when NODE_EXTRA_CA_CERTS adds an extra CA');
});
