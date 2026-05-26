package net.snowflake.jdbc.utils;

import java.io.StringReader;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyFactory;
import java.security.PrivateKey;
import java.security.spec.PKCS8EncodedKeySpec;
import java.util.Base64;
import lombok.AccessLevel;
import lombok.Getter;
import lombok.RequiredArgsConstructor;
import org.bouncycastle.asn1.pkcs.PrivateKeyInfo;
import org.bouncycastle.jce.provider.BouncyCastleProvider;
import org.bouncycastle.openssl.PEMKeyPair;
import org.bouncycastle.openssl.PEMParser;
import org.bouncycastle.openssl.jcajce.JceOpenSSLPKCS8DecryptorProviderBuilder;
import org.bouncycastle.operator.InputDecryptorProvider;
import org.bouncycastle.pkcs.PKCS8EncryptedPrivateKeyInfo;
import org.bouncycastle.util.io.pem.PemObject;
import org.bouncycastle.util.io.pem.PemWriter;
import org.json.JSONArray;
import org.json.JSONObject;

@Getter
@RequiredArgsConstructor(access = AccessLevel.PRIVATE)
public class PrivateKeyHelper {

  private final Path encryptedKeyFile;
  private final String password;

  public static PrivateKeyHelper fromParameters(Path keyFile) throws Exception {
    JSONObject params = TestParameters.get();

    String password = params.getString("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD");
    JSONArray keyLines = params.getJSONArray("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS");

    StringBuilder keyContent = new StringBuilder();
    for (int i = 0; i < keyLines.length(); i++) {
      keyContent.append(keyLines.getString(i)).append("\n");
    }

    Files.write(keyFile, keyContent.toString().getBytes(StandardCharsets.UTF_8));

    return new PrivateKeyHelper(keyFile, password);
  }

  public void writeUnencryptedKeyFile(Path outputFile) throws Exception {
    String pem = new String(Files.readAllBytes(encryptedKeyFile), StandardCharsets.UTF_8);
    PEMParser parser = new PEMParser(new StringReader(pem));
    Object pemObject = parser.readObject();
    parser.close();

    PrivateKeyInfo keyInfo;
    if (pemObject instanceof PKCS8EncryptedPrivateKeyInfo) {
      InputDecryptorProvider decryptor =
          new JceOpenSSLPKCS8DecryptorProviderBuilder()
              .setProvider(new BouncyCastleProvider())
              .build(password.toCharArray());
      keyInfo = ((PKCS8EncryptedPrivateKeyInfo) pemObject).decryptPrivateKeyInfo(decryptor);
    } else if (pemObject instanceof PEMKeyPair) {
      keyInfo = ((PEMKeyPair) pemObject).getPrivateKeyInfo();
    } else if (pemObject instanceof PrivateKeyInfo) {
      keyInfo = (PrivateKeyInfo) pemObject;
    } else {
      throw new IllegalArgumentException(
          "Unexpected PEM object type: " + pemObject.getClass().getName());
    }

    StringWriter sw = new StringWriter();
    PemWriter pw = new PemWriter(sw);
    pw.writeObject(new PemObject("PRIVATE KEY", keyInfo.getEncoded()));
    pw.close();
    Files.write(outputFile, sw.toString().getBytes(StandardCharsets.UTF_8));
  }

  public static PrivateKey loadUnencryptedPrivateKey(Path pemFile) throws Exception {
    String pem = new String(Files.readAllBytes(pemFile), StandardCharsets.UTF_8);
    String base64Key =
        pem.replace("-----BEGIN PRIVATE KEY-----", "")
            .replace("-----END PRIVATE KEY-----", "")
            .replaceAll("\\s+", "");
    byte[] der = Base64.getDecoder().decode(base64Key);
    return KeyFactory.getInstance("RSA").generatePrivate(new PKCS8EncodedKeySpec(der));
  }

  public String getBase64EncodedKey() throws Exception {
    byte[] keyBytes = Files.readAllBytes(encryptedKeyFile);
    return Base64.getEncoder().encodeToString(keyBytes);
  }
}
