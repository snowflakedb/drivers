const snowflake = require('snowflake-sdk');

const snowflakeAuthTestProtocol = process.env.SNOWFLAKE_AUTH_TEST_PROTOCOL;
const snowflakeAuthTestHost = process.env.SNOWFLAKE_AUTH_TEST_HOST;
const snowflakeAuthTestPort = process.env.SNOWFLAKE_AUTH_TEST_PORT;
const snowflakeAuthTestAccount = process.env.SNOWFLAKE_AUTH_TEST_ACCOUNT;
const snowflakeAuthTestRole = process.env.SNOWFLAKE_AUTH_TEST_ROLE;
const snowflakeAuthTestOktaAuth = process.env.SNOWFLAKE_AUTH_TEST_OKTA_AUTH;
const snowflakeAuthTestOktaUser = process.env.SNOWFLAKE_AUTH_TEST_OKTA_USER;
const snowflakeAuthTestOktaPass = process.env.SNOWFLAKE_AUTH_TEST_OKTA_PASS;
const snowflakeAuthTestDatabase = process.env.SNOWFLAKE_AUTH_TEST_DATABASE;
const snowflakeAuthTestWarehouse = process.env.SNOWFLAKE_AUTH_TEST_WAREHOUSE;
const snowflakeAuthTestSchema = process.env.SNOWFLAKE_AUTH_TEST_SCHEMA;
const snowflakeAuthTestSnowflakeUser = process.env.SNOWFLAKE_AUTH_TEST_SNOWFLAKE_USER;
const snowflakeAuthTestSnowflakeInternalRole = process.env.SNOWFLAKE_AUTH_TEST_INTERNAL_OAUTH_SNOWFLAKE_ROLE;

const accessUrlAuthTests = snowflakeAuthTestProtocol + '://' + snowflakeAuthTestHost + ':' +
    snowflakeAuthTestPort;

const okta =
    {
        accessUrl: accessUrlAuthTests,
        account: snowflakeAuthTestAccount,
        role: snowflakeAuthTestRole,
        host: snowflakeAuthTestHost,
        warehouse: snowflakeAuthTestWarehouse,
        database: snowflakeAuthTestDatabase,
        schema: snowflakeAuthTestSchema,
        username: snowflakeAuthTestOktaUser,
        password: snowflakeAuthTestOktaPass,
        authenticator: snowflakeAuthTestOktaAuth
    };

async function getPAT(patName) {
    const command = `alter user ${snowflakeAuthTestSnowflakeUser} add programmatic access token ${patName} ROLE_RESTRICTION = '${snowflakeAuthTestSnowflakeInternalRole}'`;
    return await connectAndExecuteCommand(command, true);
}

async function deletePAT(patName) {
    try {
        const command = `alter user ${snowflakeAuthTestSnowflakeUser} remove programmatic access token ${patName}`;
        await connectAndExecuteCommand(command, true);
    } catch (error) {
        // ignore error
    }
}

const executeCmdAsync = function (connection, sqlText, binds = undefined) {
    return new Promise((resolve, reject) => {
        connection.execute({
            sqlText,
            binds,
            complete: (err, _, rows) => err ? reject(err) : resolve(rows)
        });
    });
};

async function connectAndExecuteCommand(command, shouldReturnToken) {
    const connection = snowflake.createConnection(okta);
    await connection.connectAsync(undefined);
    const rows = await executeCmdAsync(connection, command);
    if (shouldReturnToken) {
        return rows[0]['token_secret'];
    }
    return null;
}

const args = process.argv.slice(2);
async function main() {
    snowflake.configure({ logLevel: 'OFF' });
    if (args[0] === 'deletePAT') {
        await deletePAT(args[1]);
    } else if (args[0] === 'getPAT') {
        console.log(await getPAT(args[1]));
    }
    process.exit(0);
}

main();
