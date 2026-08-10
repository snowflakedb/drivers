// DO NOT REVIEW THIS FILE.
// This is a manual test file that is going to be deleted as soon as we have test runner ready

/* eslint-disable no-console, @typescript-eslint/no-unused-vars */
import * as snowflake from '../src/index.js';
import getTestParameter from './e2e/utils/getTestParameter.js';

const connection = snowflake.createConnection({
  account: getTestParameter('SNOWFLAKE_TEST_ACCOUNT'),
  host: getTestParameter('SNOWFLAKE_TEST_HOST'),
  username: getTestParameter('SNOWFLAKE_TEST_USER'),
  warehouse: getTestParameter('SNOWFLAKE_TEST_WAREHOUSE'),
  database: getTestParameter('SNOWFLAKE_TEST_DATABASE'),
  schema: getTestParameter('SNOWFLAKE_TEST_SCHEMA'),
  role: getTestParameter('SNOWFLAKE_TEST_ROLE'),
  authenticator: 'SNOWFLAKE_JWT',
  privateKeyPass: getTestParameter('SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD'),
  privateKey: getTestParameter('SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS'),
});

function destroyConnectionAsync(connection: snowflake.Connection) {
  return new Promise<void>((resolve, reject) => {
    connection.destroy((err) => {
      if (err) {
        reject(err);
      } else {
        resolve();
      }
    });
  });
}

function executeAsync(options: snowflake.StatementOption) {
  return new Promise<{ stmt: snowflake.RowStatement; rows: unknown[] | undefined }>(
    (resolve, reject) => {
      connection.execute({
        complete: (err, stmt, rows) => {
          if (err) {
            reject(err);
          } else {
            resolve({ stmt, rows });
          }
        },
        ...options,
      });
    },
  );
}

const SQL_TEXT = `\
  SELECT CAST(NULL AS INT), CAST(23 AS INT), true
  UNION ALL
  SELECT CAST(42 AS INT), CAST(NULL AS INT), false
`;

(async () => {
  await connection.connectAsync();

  console.log('connected--------------------------------');
  // await testCancelQuery();
  await testStreaming();

  const { stmt, rows } = await executeAsync({ sqlText: SQL_TEXT });
  console.log('row count:', rows?.length);
  console.log(stmt.getQueryId(), stmt.getNumRows());

  console.log('fetching result--------------------------------');
  await new Promise<void>((resolve, reject) => {
    connection.fetchResult({
      queryId: stmt.getQueryId() as string,
      complete: (err, fetchStmt, fetchRows) => {
        if (err) {
          reject(err);
          return;
        }
        console.log('complete', fetchStmt.getQueryId(), 'row count:', fetchRows?.length);
        console.log('rows', fetchStmt.getNumRows());
        resolve();
      },
    });
  });

  console.log('destroying connection--------------------------------');
  await destroyConnectionAsync(connection);
})();

async function testStreaming() {
  const { stmt } = await executeAsync({ sqlText: SQL_TEXT, streamResult: true });
  await new Promise<void>((resolve, reject) => {
    stmt
      .streamRows()
      .on('data', (row) => {
        console.log('row', row);
      })
      .on('error', (err) => {
        reject(err);
      })
      .on('end', () => {
        console.log('streaming complete--------------------------------');
        resolve();
      });
  });
}

// TODO: check actually how same case behaves in old driver and whether we have
// proper test coverage for it.
// async function testCancelQuery() {
//   return new Promise<{
//     err: Error | undefined;
//     stmt: snowflake.RowStatement;
//     rows: unknown[] | undefined;
//   }>((resolve) => {
//     const stmt = connection.execute({
//       sqlText: `CALL SYSTEM$WAIT(5, 'SECONDS')`,
//       complete: (err, stmt, rows) => {
//         console.log('complete', err, stmt?.getQueryId(), rows);
//         resolve({ err, stmt, rows });
//       },
//     });
//
//     console.log('cancelling query--------------------------------');
//     setTimeout(() => {
//       stmt.cancel((err) => {
//         console.log('cancelled', err, stmt.getQueryId());
//       });
//     }, 2000);
//   });
// }
