package net.snowflake.client.internal.util;

import static net.snowflake.client.utils.RandomStringUtils.randomAlphaNumeric;
import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

public class SecretDetectorTest {
  @Test
  public void testMaskAWSSecret() {
    String sql =
        "copy into 's3://xxxx/test' from \n"
            + "(select seq1(), random()\n"
            + ", random(), random(), random(), random()\n"
            + ", random(), random(), random(), random()\n"
            + ", random() , random(), random(), random()\n"
            + "\tfrom table(generator(rowcount => 10000)))\n"
            + "credentials=(\n"
            + "  aws_key_id='xxdsdfsafds'\n"
            + "  aws_secret_key='safas+asfsad+safasf'\n"
            + "  )\n"
            + "OVERWRITE = TRUE \n"
            + "MAX_FILE_SIZE = 500000000 \n"
            + "HEADER = TRUE \n"
            + "FILE_FORMAT = (TYPE = PARQUET SNAPPY_COMPRESSION = TRUE )\n"
            + ";";
    String correct =
        "copy into 's3://xxxx/test' from \n"
            + "(select seq1(), random()\n"
            + ", random(), random(), random(), random()\n"
            + ", random(), random(), random(), random()\n"
            + ", random() , random(), random(), random()\n"
            + "\tfrom table(generator(rowcount => 10000)))\n"
            + "credentials=(\n"
            + "  aws_key_id='****'\n"
            + "  aws_secret_key='****'\n"
            + "  )\n"
            + "OVERWRITE = TRUE \n"
            + "MAX_FILE_SIZE = 500000000 \n"
            + "HEADER = TRUE \n"
            + "FILE_FORMAT = (TYPE = PARQUET SNAPPY_COMPRESSION = TRUE )\n"
            + ";";
    String masked = SecretDetector.maskSecrets(sql);
    assertEquals(correct, masked, "secret masked");
  }

  @Test
  public void testMaskSASToken() {
    // Initializing constants
    final String azureSasToken =
        "https://someaccounts.blob.core.windows.net/results/018b90ab-0033-"
            + "5f8e-0000-14f1000bd376_0/main/data_0_0_1?sv=2015-07-08&amp;"
            + "sig=iCvQmdZngZNW%2F4vw43j6%2BVz6fndHF5LI639QJba4r8o%3D&amp;"
            + "spr=https&amp;st=2016-04-12T03%3A24%3A31Z&amp;"
            + "se=2016-04-13T03%3A29%3A31Z&amp;srt=s&amp;ss=bf&amp;sp=rwl";

    final String maskedAzureSasToken =
        "https://someaccounts.blob.core.windows.net/results/018b90ab-0033-"
            + "5f8e-0000-14f1000bd376_0/main/data_0_0_1?sv=2015-07-08&amp;"
            + "sig=****&amp;"
            + "spr=https&amp;st=2016-04-12T03%3A24%3A31Z&amp;"
            + "se=2016-04-13T03%3A29%3A31Z&amp;srt=s&amp;ss=bf&amp;sp=rwl";

    final String s3SasToken =
        "https://somebucket.s3.amazonaws.com/vzy1-s-va_demo0/results/018b92f3"
            + "-01c2-02dd-0000-03d5000c8066_0/main/data_0_0_1?"
            + "x-amz-server-side-encryption-customer-algorithm=AES256&"
            + "response-content-encoding=gzip&AWSAccessKeyId=AKIAIOSFODNN7EXAMPLE"
            + "&Expires=1555481960&Signature=zFiRkdB9RtRRYomppVes4fQ%2ByWw%3D";

    final String maskedS3SasToken =
        "https://somebucket.s3.amazonaws.com/vzy1-s-va_demo0/results/018b92f3"
            + "-01c2-02dd-0000-03d5000c8066_0/main/data_0_0_1?"
            + "x-amz-server-side-encryption-customer-algorithm=AES256&"
            + "response-content-encoding=gzip&AWSAccessKeyId=****"
            + "&Expires=1555481960&Signature=****";

    assertEquals(
        maskedAzureSasToken,
        SecretDetector.maskSecrets(azureSasToken),
        "Azure SAS token is not masked");

    assertEquals(
        maskedS3SasToken, SecretDetector.maskSecrets(s3SasToken), "S3 SAS token is not masked");

    String randomString = randomAlphaNumeric(200);
    assertEquals(
        randomString,
        SecretDetector.maskSecrets(randomString),
        "Text without secrets is not unmodified");

    assertEquals(
        maskedAzureSasToken + maskedAzureSasToken,
        SecretDetector.maskSecrets(azureSasToken + azureSasToken),
        "Text with 2 Azure SAS tokens is not masked");

    assertEquals(
        maskedAzureSasToken + maskedAzureSasToken,
        SecretDetector.maskSecrets(azureSasToken + azureSasToken),
        "Text with 2 Azure SAS tokens is not masked");

    assertEquals(
        maskedAzureSasToken + maskedS3SasToken,
        SecretDetector.maskSecrets(azureSasToken + s3SasToken),
        "Text with Azure and S3 SAS tokens is not masked");
  }

  @Test
  public void testMaskSecrets() {
    // Text containing AWS secret and Azure SAS token
    final String sqlText =
        "create stage mystage "
            + "URL = 's3://mybucket/mypath/' "
            + "credentials = (aws_key_id = 'AKIAIOSFODNN7EXAMPLE' "
            + "aws_secret_key = 'frJIUN8DYpKDtOLCwo//yllqDzg='); "
            + "create stage mystage2 "
            + "URL = 'azure//mystorage.blob.core.windows.net/cont' "
            + "credentials = (azure_sas_token = "
            + "'?sv=2016-05-31&ss=b&srt=sco&sp=rwdl&se=2018-06-27T10:05:50Z&"
            + "st=2017-06-27T02:05:50Z&spr=https,http&"
            + "sig=bgqQwoXwxzuD2GJfagRg7VOS8hzNr3QLT7rhS8OFRLQ%3D')";

    final String maskedSqlText =
        "create stage mystage "
            + "URL = 's3://mybucket/mypath/' "
            + "credentials = (aws_key_id = '****' "
            + "aws_secret_key = '****'); "
            + "create stage mystage2 "
            + "URL = 'azure//mystorage.blob.core.windows.net/cont' "
            + "credentials = (azure_sas_token = "
            + "'?sv=2016-05-31&ss=b&srt=sco&sp=rwdl&se=2018-06-27T10:05:50Z&"
            + "st=2017-06-27T02:05:50Z&spr=https,http&"
            + "sig=****')";

    String masked = SecretDetector.maskSecrets(sqlText);
    assertEquals(maskedSqlText, masked, "Text with AWS secret and Azure SAS token is not masked");

    String randomString = randomAlphaNumeric(500);
    assertEquals(
        randomString,
        SecretDetector.maskSecrets(randomString),
        "Text without secrets is not unmodified");
  }

  @Test
  public void testMaskPasswordFromConnectionString() {
    // Since we have `&` in password regex pattern, we will have false positive masking here
    String connectionStr =
        "\"jdbc:snowflake://xxx.snowflakecomputing" + ".com/?user=xxx&password=xxxxxx&role=xxx\"";
    String maskedConnectionStr =
        "\"jdbc:snowflake://xxx.snowflakecomputing" + ".com/?user=xxx&password=**** ";
    assertEquals(
        maskedConnectionStr,
        SecretDetector.maskSecrets(connectionStr),
        "Text with password is not masked");

    connectionStr = "jdbc:snowflake://xxx.snowflakecomputing" + ".com/?user=xxx&password=xxxxxx";
    maskedConnectionStr =
        "jdbc:snowflake://xxx.snowflakecomputing" + ".com/?user=xxx&password=**** ";
    assertEquals(
        maskedConnectionStr,
        SecretDetector.maskSecrets(connectionStr),
        "Text with password is not masked");

    connectionStr = "jdbc:snowflake://xxx.snowflakecomputing" + ".com/?user=xxx&passcode=xxxxxx";
    maskedConnectionStr =
        "jdbc:snowflake://xxx.snowflakecomputing" + ".com/?user=xxx&passcode=**** ";
    assertEquals(
        maskedConnectionStr,
        SecretDetector.maskSecrets(connectionStr),
        "Text with password is not masked");

    connectionStr = "jdbc:snowflake://xxx.snowflakecomputing" + ".com/?user=xxx&passWord=xxxxxx";
    maskedConnectionStr =
        "jdbc:snowflake://xxx.snowflakecomputing" + ".com/?user=xxx&passWord=**** ";
    assertEquals(
        maskedConnectionStr,
        SecretDetector.maskSecrets(connectionStr),
        "Text with password is not masked");
  }

  @Test
  public void sasTokenFilterTest() throws Exception {
    String messageText = "\"privateKeyData\": \"aslkjdflasjf\"";

    String filteredMessageText = "\"privateKeyData\": \"XXXX\"";

    String result = SecretDetector.maskSecrets(messageText);

    assertEquals(filteredMessageText, result);
  }

  @Test
  public void testMaskParameterValue() {
    Map<String, String> testParametersMasked = new HashMap<>();
    testParametersMasked.put("passcodeInPassword", "test");
    testParametersMasked.put("passcode", "test");
    testParametersMasked.put("id_token", "test");
    testParametersMasked.put("private_key_pwd", "test");
    testParametersMasked.put("proxyPassword", "test");
    testParametersMasked.put("proxyUser", "test");
    testParametersMasked.put("privatekey", "test");
    testParametersMasked.put("private_key_base64", "test");
    testParametersMasked.put("privateKeyBase64", "test");
    testParametersMasked.put("id_token_password", "test");
    testParametersMasked.put("masterToken", "test");
    testParametersMasked.put("mfaToken", "test");
    testParametersMasked.put("password", "test");
    testParametersMasked.put("sessionToken", "test");
    testParametersMasked.put("token", "test");
    testParametersMasked.put("oauthClientId", "test");
    testParametersMasked.put("oauthClientSecret", "test");

    Map<String, String> testParametersUnmasked = new HashMap<>();
    testParametersUnmasked.put("oktausername", "test");
    testParametersUnmasked.put("authenticator", "test");
    testParametersUnmasked.put("proxyHost", "test");
    testParametersUnmasked.put("user", "test");
    testParametersUnmasked.put("private_key_file", "test");

    for (Map.Entry<String, String> entry : testParametersMasked.entrySet()) {
      assertEquals("****", SecretDetector.maskParameterValue(entry.getKey(), entry.getValue()));
    }
    for (Map.Entry<String, String> entry : testParametersUnmasked.entrySet()) {
      assertEquals("test", SecretDetector.maskParameterValue(entry.getKey(), entry.getValue()));
    }
  }

  @Test
  public void testMaskconnectionToken() {
    String connectionToken = "\"Authorization: Snowflake Token=\"XXXXXXXXXX\"\"";

    String maskedConnectionToken = "\"Authorization: Snowflake Token=\"****\"\"";

    assertEquals(
        maskedConnectionToken,
        SecretDetector.maskSecrets(connectionToken),
        "Text with connection token is not masked");

    connectionToken = "\"{\"requestType\":\"ISSUE\",\"idToken\":\"XXXXXXXX\"}\"";

    maskedConnectionToken = "\"{\"requestType\":\"ISSUE\",\"idToken\":\"****\"}\"";

    assertEquals(
        maskedConnectionToken,
        SecretDetector.maskSecrets(connectionToken),
        "Text with connection token is not masked");
  }

  @Test
  public void testMaskSnowflakeSessionTokens() {
    String loginResponse =
        "{\n"
            + "  \"data\" : {\n"
            + "  \"masterToken\" : \"ver:3-hint:22605402125-ETMsDgAAABRBRVMvQ0JDL1BLQ1M1UGFkZG==\",\n"
            + "  \"additionalAuthnData\" : { },\n"
            + "  \"token\" : \"ver:3-hint:22605402125-ETMsDgAAABRBRVMvQ0JDL1BLQ1M1UGFkZGluZw==\",\n"
            + "  \"validityInSeconds\" : 3599\n"
            + "  }\n"
            + "}";

    String masked = SecretDetector.maskSecrets(loginResponse);

    assertEquals(-1, masked.indexOf("ETMsDg"), "masterToken value should be masked");
    assertEquals(-1, masked.indexOf("ver:3-hint"), "token value should be masked");
    assertEquals(
        true,
        masked.contains("masterToken\" : \"****"),
        "masterToken should be replaced with ****");
    assertEquals(true, masked.contains("token\" : \"****"), "token should be replaced with ****");
  }

  @Test
  public void testMaskOAuthSecrets() {
    String tokenResponseJson =
        "{\n"
            + "  \"access_token\" : \"some:FAKE_token123\",\n"
            + "  \"refresh_token\" : \"some:FAKE_token123\",\n"
            + "  \"token_type\" : \"Bearer\",\n"
            + "  \"username\" : \"OAUTH_TEST_AUTH_CODE\",\n"
            + "  \"scope\" : \"refresh_token session:role:ANALYST\",\n"
            + "  \"expires_in\" : 600,\n"
            + "  \"refresh_token_expires_in\" : 86399,\n"
            + "  \"idpInitiated\" : false\n"
            + "}";

    String maskedTokenResponseJson =
        "{\n"
            + "  \"access_token\":\"****\",\n"
            + "  \"refresh_token\":\"****\",\n"
            + "  \"token_type\" : \"Bearer\",\n"
            + "  \"username\" : \"OAUTH_TEST_AUTH_CODE\",\n"
            + "  \"scope\" : \"refresh_token session:role:ANALYST\",\n"
            + "  \"expires_in\" : 600,\n"
            + "  \"refresh_token_expires_in\" : 86399,\n"
            + "  \"idpInitiated\" : false\n"
            + "}";

    assertEquals(
        maskedTokenResponseJson,
        SecretDetector.maskSecrets(tokenResponseJson),
        "Text with OAuth tokens is not masked");
  }

  @Test
  public void testEncryptionMaterialFilter() throws Exception {
    String messageText =
        "{\"data\":"
            + "{\"autoCompress\":true,"
            + "\"overwrite\":false,"
            + "\"clientShowEncryptionParameter\":true,"
            + "\"encryptionMaterial\":{\"queryStageMasterKey\":\"asdfasdfasdfasdf==\",\"queryId\":\"01b6f5ba-0002-0181-0000-11111111da\",\"smkId\":1111},"
            + "\"stageInfo\":{\"locationType\":\"AZURE\", \"region\":\"eastus2\"}";

    String filteredMessageText =
        "{\"data\":"
            + "{\"autoCompress\":true,"
            + "\"overwrite\":false,"
            + "\"clientShowEncryptionParameter\":true,"
            + "\"encryptionMaterial\" : ****,"
            + "\"stageInfo\":{\"locationType\":\"AZURE\", \"region\":\"eastus2\"}";

    String result = SecretDetector.filterEncryptionMaterial(messageText);

    assertEquals(filteredMessageText, result);
  }
}
