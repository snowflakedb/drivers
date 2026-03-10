import * as fs from "fs";
import * as path from "path";
import * as snowflake from "./snowflake";

const PARAM_KEY_MAP: Record<string, string> = {
  SNOWFLAKE_TEST_ACCOUNT: "account",
  SNOWFLAKE_TEST_USER: "username",
  SNOWFLAKE_TEST_PASSWORD: "password",
  SNOWFLAKE_TEST_HOST: "host",
  SNOWFLAKE_TEST_DATABASE: "database",
  SNOWFLAKE_TEST_SCHEMA: "schema",
  SNOWFLAKE_TEST_WAREHOUSE: "warehouse",
  SNOWFLAKE_TEST_ROLE: "role",
};

function loadConnectionOptions(): snowflake.ConnectionOptions {
  const parametersPath = path.resolve(__dirname, "../../parameters.json");
  const raw = JSON.parse(fs.readFileSync(parametersPath, "utf-8"));
  const testParams: Record<string, string> = raw.testconnection ?? {};

  const opts: Record<string, string> = {
    authenticator: "SNOWFLAKE_PASSWORD",
  };
  for (const [envKey, optKey] of Object.entries(PARAM_KEY_MAP)) {
    if (testParams[envKey]) {
      opts[optKey] = testParams[envKey];
    }
  }
  return opts as unknown as snowflake.ConnectionOptions;
}

async function main() {
  snowflake.configure({ logLevel: "DEBUG" });

  const connection = snowflake.createConnection(loadConnectionOptions());

  try {
    await connection.connect();
    console.log("Connected successfully!");

    const result = await connection.execute({
      sqlText: "select 1",
    });
    console.log("Query result:", result.rows);
    console.log("Columns:", result.columns);
    console.log("Query ID:", result.queryId);
  } catch (err) {
    console.error("Failed:", err);
    process.exit(1);
  }
}

main();
