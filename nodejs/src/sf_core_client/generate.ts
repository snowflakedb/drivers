import * as protobuf from "protobufjs";
import * as path from "path";
import * as fs from "fs";

const PROTO_PATH = path.resolve(
  __dirname,
  "../../../protobuf/database_driver_v1.proto",
);
const PROTOBUFJS_ROOT = path.resolve(
  __dirname,
  "../../node_modules/protobufjs",
);
const OUTPUT_PATH = path.resolve(__dirname, "generated/client.ts");
const SERVICE_FQN = "database_driver_v1.DatabaseDriver";

function pascalToSnake(name: string): string {
  return name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}

function pascalToCamel(name: string): string {
  return name.charAt(0).toLowerCase() + name.slice(1);
}

async function main() {
  const root = new protobuf.Root();
  root.resolvePath = (_origin: string, target: string) => {
    if (target.startsWith("google/")) {
      return path.resolve(PROTOBUFJS_ROOT, target);
    }
    return path.resolve(path.dirname(PROTO_PATH), target);
  };
  await root.load(PROTO_PATH);

  const service = root.lookupService(SERVICE_FQN);

  const lines: string[] = [];
  lines.push(
    '// Auto-generated from database_driver_v1.proto -- DO NOT EDIT',
  );
  lines.push('import { callProto } from "../transport";');
  lines.push('import { database_driver_v1 as proto } from "./proto";');
  lines.push("");
  lines.push("export class SfCoreClient {");

  for (const method of service.methodsArray) {
    const snakeName = pascalToSnake(method.name);
    const camelName = pascalToCamel(method.name);
    const reqType = method.requestType;
    const resType = method.responseType;

    lines.push(`  async ${camelName}(`);
    lines.push(`    request: proto.I${reqType},`);
    lines.push(`  ): Promise<proto.${resType}> {`);
    lines.push(`    return callProto<proto.I${reqType}, proto.${resType}>(`);
    lines.push(`      "${snakeName}",`);
    lines.push(`      "${reqType}",`);
    lines.push(`      "${resType}",`);
    lines.push(`      request,`);
    lines.push(`    );`);
    lines.push(`  }`);
    lines.push("");
  }

  lines.push("}");
  lines.push("");

  const outputDir = path.dirname(OUTPUT_PATH);
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  fs.writeFileSync(OUTPUT_PATH, lines.join("\n"));
  console.log(`Generated ${OUTPUT_PATH}`);
  console.log(`  ${service.methodsArray.length} methods`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
