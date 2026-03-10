import * as snowflake from "./snowflake";

async function main() {
  snowflake.configure({ logLevel: "DEBUG" });

  const connection = snowflake.createConnection({});

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
