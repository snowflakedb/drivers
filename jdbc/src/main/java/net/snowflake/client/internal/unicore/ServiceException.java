package net.snowflake.client.internal.unicore;

import com.google.protobuf.Message;

/**
 * Thrown when a core RPC returns a service-level (application) error. Carries the decoded error
 * payload. The JDBC transport layer wraps it in a {@link
 * net.snowflake.client.internal.api.implementation.exception.CoreException} carrier.
 *
 * <p>Lives in the transport layer (rather than nested in a generated service interface) so it can
 * be shared by the generated clients and by {@link ResponseDecoder} / {@link CoreFuture}.
 */
public class ServiceException extends RuntimeException {
  public final transient Message error;

  public ServiceException(Message error) {
    super(error.toString());
    this.error = error;
  }
}
