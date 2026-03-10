import * as snowflake from "./snowflake";

async function main() {
  snowflake.configure({ logLevel: "DEBUG" });

  const connection = snowflake.createConnection({
    // TODO
  });

  try {
    await connection.connect();
    console.log("Connected successfully!");
  } catch (err) {
    console.error("Connection failed:", err);
    process.exit(1);
  }
}

main();
