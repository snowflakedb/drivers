import { Buffer } from 'node:buffer';

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'ANY';

export interface QueryParameterMatcher {
  equalTo?: string;
  matches?: string;
}

export interface BodyPattern {
  matchesJsonPath?: string;
  equalToJson?: string;
  equalTo?: string;
  contains?: string;
}

export interface HeaderMatcher {
  equalTo?: string;
  contains?: string;
}

export interface RequestPattern {
  method: HttpMethod;
  urlPathPattern?: string;
  urlPath?: string;
  urlPattern?: string;
  queryParameters?: Record<string, QueryParameterMatcher>;
  headers?: Record<string, HeaderMatcher>;
  bodyPatterns?: BodyPattern[];
}

export interface ResponseDefinition {
  status?: number;
  headers?: Record<string, string>;
  jsonBody?: unknown;
  body?: string;
  proxyBaseUrl?: string;
  transformers?: string[];
  fixedDelayMilliseconds?: number;
  fault?:
    | 'CONNECTION_RESET_BY_PEER'
    | 'EMPTY_RESPONSE'
    | 'MALFORMED_RESPONSE_CHUNK'
    | 'RANDOM_DATA_THEN_CLOSE';
}

export interface StubMapping {
  scenarioName?: string;
  requiredScenarioState?: string;
  newScenarioState?: string;
  priority?: number;
  request: RequestPattern;
  response: ResponseDefinition;
}

export function proxyAllTo(proxyBaseUrl: string): StubMapping {
  return {
    priority: 10,
    request: {
      method: 'ANY',
      urlPattern: '.*',
    },
    response: { proxyBaseUrl },
  };
}

export function jsonResponse(status: number, body: unknown): ResponseDefinition {
  return {
    status,
    headers: { 'Content-Type': 'application/json' },
    jsonBody: body,
  };
}

export function resultWithOneRemoteChunk(queryId: string, chunkUrl: string): ResponseDefinition {
  return jsonResponse(200, {
    success: true,
    data: {
      queryId,
      rowtype: [
        {
          name: 'C1',
          type: 'text',
          nullable: true,
          length: 16777216,
          byteLength: 16777216,
          precision: null,
          scale: null,
        },
      ],
      rowset: [],
      total: 1,
      returned: 1,
      queryResultFormat: 'json',
      statementTypeId: 4096,
      chunkHeaders: {},
      qrmk: 'bW9jaw==',
      chunks: [{ url: chunkUrl, rowCount: 1, uncompressedSize: 8, compressedSize: 8 }],
    },
  });
}

export function oauthTokenRequestSuccess(): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPath: '/oauth/token-request',
    },
    response: jsonResponse(200, {
      access_token: 'ac-access-token-success',
      refresh_token: 'ac-refresh-token-success',
      token_type: 'Bearer',
      expires_in: 600,
    }),
  };
}

export function loginSuccess(dataOverrides: Record<string, unknown> = {}): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPathPattern: '/session/v1/login-request.*',
    },
    response: jsonResponse(200, {
      success: true,
      data: {
        token: 'mock_session_token',
        masterToken: 'mock_master_token',
        sessionId: 12345,
        validityInSeconds: 3600,
        masterValidityInSeconds: 14400,
        sessionInfo: {
          databaseName: '',
          schemaName: '',
          warehouseName: '',
          roleName: '',
        },
        parameters: [{ name: 'CLIENT_TELEMETRY_ENABLED', value: false }],
        ...dataOverrides,
      },
    }),
  };
}

export function delayedLoginSuccess(delayMs: number): StubMapping {
  const mapping = loginSuccess();
  return {
    ...mapping,
    response: {
      ...mapping.response,
      fixedDelayMilliseconds: delayMs,
    },
  };
}

export function delayedLoginFailure(delayMs: number): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPathPattern: '/session/v1/login-request.*',
    },
    response: {
      ...jsonResponse(200, {
        success: false,
        code: '390100',
        message: 'Incorrect username or password was specified.',
      }),
      fixedDelayMilliseconds: delayMs,
    },
  };
}

export function queryRequestFail(code: string, message: string): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPathPattern: '/queries/v1/query-request.*',
    },
    response: jsonResponse(200, { success: false, code, message }),
  };
}

export function tokenRequestFail(code: string, message: string): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPath: '/session/token-request',
    },
    response: jsonResponse(200, { success: false, code, message }),
  };
}

export function tokenRequestConnectionReset(): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPath: '/session/token-request',
    },
    response: { fault: 'CONNECTION_RESET_BY_PEER' },
  };
}

export function logoutSuccess(): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPath: '/session',
      queryParameters: { delete: { equalTo: 'true' } },
    },
    response: jsonResponse(200, { success: true }),
  };
}

export function heartbeatSuccess(): StubMapping {
  return {
    priority: 1,
    request: {
      method: 'POST',
      urlPathPattern: '/session/heartbeat.*',
    },
    response: jsonResponse(200, { success: true }),
  };
}

export function telemetrySuccess(): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPathPattern: '/telemetry/send.*',
    },
    response: jsonResponse(200, { success: true }),
  };
}

export function authenticatorRequestSuccess(): StubMapping {
  return {
    request: {
      method: 'POST',
      urlPathPattern: '/session/authenticator-request.*',
    },
    response: {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
      transformers: ['response-template'],
      jsonBody: {
        success: true,
        data: {
          ssoUrl:
            "https://idp.snowflake.com/sso?browser_mode_redirect_port={{jsonPath request.body '$.data.BROWSER_MODE_REDIRECT_PORT'}}",
          proofKey: 'test_proof_key',
        },
      },
    },
  };
}

export function monitoringQueryStatus(
  queryId: string,
  status: string,
  queryFields: Record<string, unknown> = {},
): StubMapping {
  return {
    priority: 1,
    request: {
      method: 'GET',
      urlPathPattern: `/monitoring/queries/+${queryId}.*`,
    },
    response: jsonResponse(200, {
      success: true,
      data: {
        queries: [{ status, id: queryId, ...queryFields }],
      },
    }),
  };
}

export function monitoringQueryFailure(
  queryId: string,
  httpStatus: number,
  body: unknown,
): StubMapping {
  return {
    priority: 1,
    request: {
      method: 'GET',
      urlPathPattern: `/monitoring/queries/+${queryId}.*`,
    },
    response: jsonResponse(httpStatus, body),
  };
}

export function monitoringQueryStatuses(
  queryId: string,
  statuses: readonly string[],
): StubMapping[] {
  const scenarioName = `query-status-${queryId}`;
  return statuses.map((status, index) => {
    const mapping = monitoringQueryStatus(queryId, status);
    mapping.scenarioName = scenarioName;
    mapping.requiredScenarioState = index === 0 ? 'Started' : `s${index}`;
    if (index < statuses.length - 1) {
      mapping.newScenarioState = `s${index + 1}`;
    }
    return mapping;
  });
}

const CLIENT_CREDENTIALS_FORM_BODY_FRAGMENT =
  'grant_type=client_credentials&client_id=123&client_secret=clientSecret';

export function clientCredentialsTokenStubs(
  status: number,
  body: unknown,
  expectedScope?: string,
): StubMapping[] {
  const response = jsonResponse(status, body);
  const scopePatterns: BodyPattern[] =
    expectedScope === undefined ? [] : [{ contains: `scope=${encodeURIComponent(expectedScope)}` }];
  return [
    {
      request: {
        method: 'POST',
        urlPathPattern: '/oauth/token-request.*',
        headers: {
          'Content-Type': {
            contains: 'application/x-www-form-urlencoded;charset=UTF-8',
          },
        },
        bodyPatterns: [{ contains: CLIENT_CREDENTIALS_FORM_BODY_FRAGMENT }, ...scopePatterns],
      },
      response,
    },
    {
      request: {
        method: 'POST',
        urlPathPattern: '/oauth/token-request.*',
        headers: {
          'Content-Type': { contains: 'application/x-www-form-urlencoded' },
          Authorization: { equalTo: `Basic ${Buffer.from('123:clientSecret').toString('base64')}` },
        },
        bodyPatterns: [{ contains: 'grant_type=client_credentials' }, ...scopePatterns],
      },
      response,
    },
  ];
}
