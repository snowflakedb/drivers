import * as protobuf from "protobufjs";
import * as path from "path";
import Long from "long";

const PROTO_PATH = path.resolve(
  __dirname,
  "../../../protobuf/database_driver_v1.proto",
);
const PROTOBUFJS_ROOT = path.resolve(
  __dirname,
  "../../node_modules/protobufjs",
);
const PACKAGE = "database_driver_v1";

let cachedRoot: protobuf.Root | null = null;

async function getRoot(): Promise<protobuf.Root> {
  if (!cachedRoot) {
    const newRoot = new protobuf.Root();
    newRoot.resolvePath = (_origin: string, target: string) => {
      // google/protobuf/* lives inside the protobufjs npm package
      if (target.startsWith("google/")) {
        return path.resolve(PROTOBUFJS_ROOT, target);
      }
      return path.resolve(path.dirname(PROTO_PATH), target);
    };
    await newRoot.load(PROTO_PATH);
    cachedRoot = newRoot;
  }
  return cachedRoot;
}

function longsToBigInt(obj: unknown): unknown {
  if (Long.isLong(obj)) {
    return BigInt(obj.toString());
  }
  if (Array.isArray(obj)) {
    return obj.map(longsToBigInt);
  }
  if (obj !== null && typeof obj === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
      out[k] = longsToBigInt(v);
    }
    return out;
  }
  return obj;
}

function bigIntsToLong(obj: unknown): unknown {
  if (typeof obj === "bigint") {
    return Long.fromString(obj.toString());
  }
  if (Array.isArray(obj)) {
    return obj.map(bigIntsToLong);
  }
  if (obj !== null && typeof obj === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
      out[k] = bigIntsToLong(v);
    }
    return out;
  }
  return obj;
}

export async function encodeMessage(
  typeName: string,
  obj: object,
): Promise<Uint8Array> {
  const root = await getRoot();
  const MessageType = root.lookupType(`${PACKAGE}.${typeName}`);
  const converted = bigIntsToLong(obj) as Record<string, unknown>;
  const errMsg = MessageType.verify(converted);
  if (errMsg) throw new Error(`Invalid ${typeName}: ${errMsg}`);
  const message = MessageType.create(converted);
  return MessageType.encode(message).finish();
}

export async function decodeMessage<T = Record<string, unknown>>(
  typeName: string,
  buffer: Uint8Array,
): Promise<T> {
  const root = await getRoot();
  const MessageType = root.lookupType(`${PACKAGE}.${typeName}`);
  const message = MessageType.decode(buffer);
  const plain = MessageType.toObject(message, {
    enums: String,
    defaults: true,
  });
  return longsToBigInt(plain) as T;
}
