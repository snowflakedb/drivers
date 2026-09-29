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

export interface RequestPattern {
  method: HttpMethod;
  urlPathPattern?: string;
  urlPath?: string;
  urlPattern?: string;
  queryParameters?: Record<string, QueryParameterMatcher>;
  bodyPatterns?: BodyPattern[];
}

export interface ResponseDefinition {
  status?: number;
  headers?: Record<string, string>;
  jsonBody?: unknown;
  proxyBaseUrl?: string;
  transformers?: string[];
  fixedDelayMilliseconds?: number;
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
