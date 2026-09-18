package net.snowflake.client.internal.api.implementation.connection;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.CharBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.function.Function;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.parameters.ParameterKeyNormalizer;

/** Parser and profile selector for the JDBC auto connection URL. */
public final class AutoConnectionUrl {
  public static final String PREFIX = "jdbc:snowflake:auto";
  private static final String CONNECTION_NAME = "connectionName";
  private static final String DEFAULT_CONNECTION_NAME_ENV = "SNOWFLAKE_DEFAULT_CONNECTION_NAME";

  private AutoConnectionUrl() {}

  /**
   * Returns whether the URL is the auto prefix or an auto-prefixed query/fragment candidate. The
   * parser performs strict syntax validation, including rejecting fragments.
   */
  public static boolean isAutoConnectionUrl(String url) {
    return PREFIX.equals(url)
        || (url != null && (url.startsWith(PREFIX + "?") || url.startsWith(PREFIX + "#")));
  }

  /** Parses and URL-decodes auto connection query parameters. */
  public static Map<String, String> parseParameters(String url) {
    requireAutoConnectionUrl(url);
    rejectFragment(url);

    int queryStart = url.indexOf('?');
    if (queryStart < 0) {
      return Collections.emptyMap();
    }
    if (queryStart == url.length() - 1) {
      throw invalidParameter("JDBC auto connection URL contains an empty query");
    }
    return parseQuery(url.substring(queryStart + 1));
  }

  private static void requireAutoConnectionUrl(String url) {
    if (!isAutoConnectionUrl(url)) {
      // The URL itself stays out of the message: a caller that reaches this branch passed a
      // non-auto URL, whose query string carries credentials such as password or token.
      throw invalidParameter("Invalid JDBC auto connection URL; expected prefix " + PREFIX);
    }
  }

  private static void rejectFragment(String url) {
    if (url.indexOf('#') >= 0) {
      throw invalidParameter("JDBC auto connection URLs must not contain fragments");
    }
  }

  private static Map<String, String> parseQuery(String query) {
    Map<String, String> parameters = new LinkedHashMap<>();
    Set<String> canonicalKeys = new HashSet<>();
    String[] pairs = query.split("&", -1);
    for (int index = 0; index < pairs.length; index++) {
      putQueryParameter(parameters, canonicalKeys, pairs[index], index + 1);
    }
    return Collections.unmodifiableMap(parameters);
  }

  private static void putQueryParameter(
      Map<String, String> parameters, Set<String> canonicalKeys, String pair, int componentNumber) {
    if (pair.isEmpty()) {
      throw invalidParameter("JDBC auto connection URL contains an empty query component");
    }
    String[] keyValue = pair.split("=", 2);
    if (keyValue.length != 2) {
      // The component is identified by position rather than content: without a literal '=' the
      // whole component is one opaque token, which a percent-encoded separator turns into a
      // readable credential.
      throw invalidParameter(
          "JDBC auto connection URL query component "
              + componentNumber
              + " must contain '=': parameter name and value are not separated");
    }
    try {
      String key = decodeComponent(keyValue[0]);
      String value = decodeComponent(keyValue[1]);
      validateParameterName(key);
      registerParameter(parameters, canonicalKeys, key, value, componentNumber);
    } catch (CharacterCodingException | IllegalArgumentException e) {
      throw invalidParameter("Invalid URL encoding in JDBC auto connection URL", e);
    }
  }

  private static void validateParameterName(String key) {
    if (key.isEmpty()) {
      throw invalidParameter("JDBC auto connection URL contains an empty parameter name");
    }
    if (key.trim().isEmpty()) {
      throw invalidParameter("JDBC auto connection URL contains a whitespace-only parameter name");
    }
  }

  private static void registerParameter(
      Map<String, String> parameters,
      Set<String> canonicalKeys,
      String key,
      String value,
      int componentNumber) {
    if (value.isEmpty() && !isConnectionName(key)) {
      return;
    }
    if (!canonicalKeys.add(canonicalKey(key))) {
      throw invalidParameter(
          "JDBC auto connection URL query component "
              + componentNumber
              + " duplicates an earlier parameter after alias normalization");
    }
    parameters.put(key, value);
  }

  public static String selectConfiguredConnectionName(
      Map<String, String> parameters, Function<String, String> environment) {
    String fromUrl = null;
    for (Map.Entry<String, String> entry : parameters.entrySet()) {
      if (isConnectionName(entry.getKey())) {
        fromUrl = entry.getValue();
        break;
      }
    }
    if (!isBlank(fromUrl)) {
      return fromUrl;
    }

    String fromEnvironment = environment.apply(DEFAULT_CONNECTION_NAME_ENV);
    return isBlank(fromEnvironment) ? null : fromEnvironment;
  }

  private static boolean isBlank(String value) {
    return value == null || value.trim().isEmpty();
  }

  public static boolean isConnectionName(String key) {
    return CONNECTION_NAME.equalsIgnoreCase(key) || "connection_name".equalsIgnoreCase(key);
  }

  public static String canonicalKey(String key) {
    if (isConnectionName(key)) {
      return "connection_name";
    }
    return ParameterKeyNormalizer.normalize(key).toLowerCase(Locale.ROOT);
  }

  private static String decodeComponent(String component) throws CharacterCodingException {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream(component.length());
    int index = 0;
    while (index < component.length()) {
      char current = component.charAt(index);
      if (current == '%') {
        if (index + 2 >= component.length()) {
          throw new IllegalArgumentException("Incomplete percent escape");
        }
        int high = Character.digit(component.charAt(index + 1), 16);
        int low = Character.digit(component.charAt(index + 2), 16);
        if (high < 0 || low < 0) {
          throw new IllegalArgumentException("Malformed percent escape");
        }
        bytes.write((high << 4) + low);
        index += 3;
      } else if (current == '+') {
        bytes.write(' ');
        index++;
      } else {
        int runEnd = index + 1;
        while (runEnd < component.length()) {
          char candidate = component.charAt(runEnd);
          if (candidate == '%' || candidate == '+') {
            break;
          }
          runEnd++;
        }
        ByteBuffer encoded =
            StandardCharsets.UTF_8
                .newEncoder()
                .onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT)
                .encode(CharBuffer.wrap(component, index, runEnd));
        while (encoded.hasRemaining()) {
          bytes.write(encoded.get());
        }
        index = runEnd;
      }
    }
    return StandardCharsets.UTF_8
        .newDecoder()
        .onMalformedInput(CodingErrorAction.REPORT)
        .onUnmappableCharacter(CodingErrorAction.REPORT)
        .decode(ByteBuffer.wrap(bytes.toByteArray()))
        .toString();
  }

  private static SFSQLException invalidParameter(String message) {
    return new SFSQLException(ErrorCode.INVALID_PARAMETER_VALUE, message);
  }

  private static SFSQLException invalidParameter(String message, Throwable cause) {
    return new SFSQLException(
        ErrorCode.INVALID_PARAMETER_VALUE, message + ": " + cause.getMessage());
  }
}
