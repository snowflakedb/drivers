const axios = require('axios');
const assert = require('assert');

const snowflakeAuthTestOauthUrl = process.env.SNOWFLAKE_AUTH_TEST_OAUTH_URL;
const snowflakeAuthTestOauthClientId = process.env.SNOWFLAKE_AUTH_TEST_OAUTH_CLIENT_ID;
const snowflakeAuthTestOauthClientSecret = process.env.SNOWFLAKE_AUTH_TEST_OAUTH_CLIENT_SECRET;
const snowflakeAuthTestOktaUser = process.env.SNOWFLAKE_AUTH_TEST_OKTA_USER;
const snowflakeAuthTestOktaPass = process.env.SNOWFLAKE_AUTH_TEST_OKTA_PASS;
const snowflakeAuthTestRole = process.env.SNOWFLAKE_AUTH_TEST_ROLE;


async function getToken() {
    const response =  await axios.post(snowflakeAuthTestOauthUrl, data, {
        headers: {
            'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8'
        },
        auth: {
            username: snowflakeAuthTestOauthClientId,
            password: snowflakeAuthTestOauthClientSecret
        }
    });
    assert.strictEqual(response.status, 200, 'Failed to get access token');
    return response.data.access_token;
}

const data = [
    `username=${snowflakeAuthTestOktaUser}`,
    `password=${snowflakeAuthTestOktaPass}`,
    'grant_type=password',
    `scope=session:role:${snowflakeAuthTestRole.toLowerCase()}`
].join('&');

async function main() {
    const token = await getToken();
    console.log(token);
    return token;
}
main();
