import * as path from "path";
import { encodeMessage, decodeMessage } from "./proto";

interface NativeAddon {
  apiCallProto(
    api: string,
    method: string,
    request: Buffer,
  ): { code: number; data: Buffer };
  initLogger(
    callback: (
      level: number,
      message: string,
      filename: string,
      line: number,
      func: string,
    ) => void,
  ): number;
}

const addon: NativeAddon = require(
  path.resolve(__dirname, "../../build/Release/sf_core_napi.node"),
);

const API_NAME = "DatabaseDriver";

export class DriverError extends Error {
  public readonly statusCode: string;
  public readonly vendorCode?: number;
  public readonly sqlState?: string;

  constructor(
    message: string,
    statusCode: string,
    vendorCode?: number,
    sqlState?: string,
  ) {
    super(message);
    this.name = "DriverError";
    this.statusCode = statusCode;
    this.vendorCode = vendorCode;
    this.sqlState = sqlState;
  }
}

export async function callProto<
  TReq extends object = Record<string, unknown>,
  TResp = Record<string, unknown>,
>(
  method: string,
  requestType: string,
  responseType: string,
  requestObj: TReq,
): Promise<TResp> {
  const requestBytes = await encodeMessage(requestType, requestObj);
  const result = addon.apiCallProto(
    API_NAME,
    method,
    Buffer.from(requestBytes),
  );

  if (result.code === 0) {
    return decodeMessage<TResp>(responseType, result.data);
  }

  if (result.code === 1) {
    const exception = await decodeMessage<{
      rootCause?: string;
      message?: string;
      statusCode: string;
      vendorCode?: number;
      sqlState?: string;
    }>("DriverException", result.data);
    const message =
      exception.rootCause ||
      exception.message ||
      "Unknown driver error";
    throw new DriverError(
      message,
      exception.statusCode,
      exception.vendorCode,
      exception.sqlState,
    );
  }

  // code === 2: transport error (raw string)
  const errorText = result.data.toString("utf-8");
  throw new Error(`Transport error: ${errorText}`);
}

export function initNativeLogger(
  callback: (
    level: number,
    message: string,
    filename: string,
    line: number,
    func: string,
  ) => void,
): number {
  return addon.initLogger(callback);
}
