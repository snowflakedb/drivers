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
  queryParameters?: Record<string, QueryParameterMatcher>;
  bodyPatterns?: BodyPattern[];
}

export interface ResponseDefinition {
  status: number;
  headers?: Record<string, string>;
  jsonBody?: unknown;
}

export interface StubMapping {
  scenarioName?: string;
  requiredScenarioState?: string;
  newScenarioState?: string;
  priority?: number;
  request: RequestPattern;
  response: ResponseDefinition;
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
