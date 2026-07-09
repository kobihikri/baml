import initWasm, {
  callFunction as callWasmFunction,
  callFunctionSync as callWasmFunctionSync,
  cancelFunctionCall as cancelWasmFunctionCall,
  completeWebHostCall,
  mintWebHostValueKey,
  newFunctionCall as newWasmFunctionCall,
  registerWebHostCallable,
  registerWebHostValueReleaseCallback,
  releaseWebHostCallable,
  stageRuntimeBytecode,
} from "./wasm/bridge_web_core.js";

await initWasm();

async function browserFetch(
  _callId: number,
  method: string,
  url: string,
  headersJson: string,
  body: string,
): Promise<{ status: number; headersJson: string; url: string; bodyPromise: Promise<string> }> {
  const headers = JSON.parse(headersJson) as Record<string, string>;
  const response = await fetch(url, {
    method,
    headers,
    body: body.length === 0 || method === "GET" || method === "HEAD" ? undefined : body,
  });
  const responseHeaders: Record<string, string> = {};
  response.headers.forEach((value, key) => { responseHeaders[key] = value; });
  return {
    status: response.status,
    headersJson: JSON.stringify(responseHeaders),
    url: response.url,
    bodyPromise: response.text(),
  };
}

const unsupportedProcess = async (): Promise<never> => {
  throw new Error("process execution is not available in a browser");
};

const browserCallbacks = {
  fetch: browserFetch,
  env: (key: string): string | undefined => (globalThis as typeof globalThis & { BAML_ENV?: Record<string, string> }).BAML_ENV?.[key],
  input: async (_requestId: number, promptText?: string): Promise<string> => globalThis.prompt?.(promptText ?? "") ?? "",
  exec: unsupportedProcess,
  shell: unsupportedProcess,
  lsp_send_notification: () => {},
  lsp_send_response: () => {},
  lsp_make_request: () => {},
  playground_send_notification: () => {},
  host_dispatch: (key: bigint): never => { throw new Error(`unknown browser host callable ${key}`); },
};

export interface HandleKey { low: number; high: number; }

function keyFromBigint(value: bigint): HandleKey {
  return { low: Number(BigInt.asIntN(32, value)), high: Number(BigInt.asIntN(32, value >> 32n)) };
}

export class BamlHandle {
  constructor(public readonly key: HandleKey, public readonly handleType: number) {}
  clone(): BamlHandle { return new BamlHandle(this.key, this.handleType); }
  _cloneKeyForWire(): HandleKey { return this.key; }
}

class BamlMedia {
  protected constructor(private readonly handle: BamlHandle) {}
  static fromUrl(_url: string, _mimeType?: string | null): BamlMedia { throw new Error("browser media construction is not implemented yet"); }
  static fromFile(_file: string, _mimeType?: string | null): BamlMedia { throw new Error("browser media construction is not implemented yet"); }
  static fromBase64(_base64: string, _mimeType?: string | null): BamlMedia { throw new Error("browser media construction is not implemented yet"); }
  static _fromHandle(handle: BamlHandle): BamlMedia { return new this(handle); }
  _toHandle(): BamlHandle { return this.handle.clone(); }
  url(): string | null { return null; }
  file(): string | null { return null; }
  base64(): string { throw new Error("browser media access is not implemented yet"); }
  mimeType(): string | null { return null; }
}

export class BamlImage extends BamlMedia {}
export class BamlAudio extends BamlMedia {}
export class BamlVideo extends BamlMedia {}
export class BamlPdf extends BamlMedia {}

export class BamlCallContext {
  private callIds = new Set<bigint>();
  private isAborted = false;
  abort(): void {
    this.isAborted = true;
    for (const callId of this.callIds) cancelWasmFunctionCall(callId);
  }
  get aborted(): boolean { return this.isAborted; }
  _attachCallId(callId: string): void {
    const id = BigInt(callId);
    this.callIds.add(id);
    if (this.isAborted) cancelWasmFunctionCall(id);
  }
  _detachCallId(callId: string): void { this.callIds.delete(BigInt(callId)); }
}

export class HostSpanManager {
  enter(_name: string, _args: unknown): void {}
  exitOk(): void {}
  exitError(_message: string): void {}
  upsertTags(_tags: Record<string, string>): void {}
  deepClone(): HostSpanManager { return new HostSpanManager(); }
  contextDepth(): number { return 0; }
}

export class Timing {}
export class Usage {}
export class LlmCall {}
export { LlmCall as LLMCall };
export class FunctionLog {}
export class Collector {
  constructor(_name?: string | null) {}
}

export class BamlRuntime {
  static initializeRuntimeFromBytecode(bytecode: Uint8Array): BamlRuntime {
    stageRuntimeBytecode(bytecode, browserCallbacks);
    runtime = new BamlRuntime();
    return runtime;
  }
  static initializeRuntime(_rootPath: string, _files: Record<string, string>): BamlRuntime {
    throw new Error("browser source initialization is not implemented; use bytecode");
  }
  callFunctionSync(functionName: string, encodedArgs: Uint8Array, _ctx?: HostSpanManager | null, _collectors?: Collector[] | null): Uint8Array {
    return callWasmFunctionSync(functionName, encodedArgs);
  }
  callFunction(functionName: string, encodedArgs: Uint8Array, _ctx?: HostSpanManager | null, _collectors?: Collector[] | null): Promise<Uint8Array> {
    return callWasmFunction(functionName, encodedArgs);
  }
}

let runtime: BamlRuntime | undefined;
let hostRelease: ((key: HandleKey) => void) | undefined;

export function getRuntime(): BamlRuntime {
  if (!runtime) throw new Error("BAML runtime has not been initialized");
  return runtime;
}
export function newFunctionCall(): bigint { return newWasmFunctionCall(); }
export function cancelFunctionCall(callId: bigint | string): boolean { return cancelWasmFunctionCall(BigInt(callId)); }
export function flushEvents(): void {}
export function getVersion(): string { return "0.0.0-web"; }
export function mintHostValueKey(): HandleKey { return keyFromBigint(mintWebHostValueKey()); }
export function registerHostValueReleaseCallback(callback: (key: HandleKey) => void): void {
  hostRelease = callback;
  registerWebHostValueReleaseCallback((key: bigint) => hostRelease?.(keyFromBigint(key)));
}
export function registerHostCallable(callback: (callId: number, args: Uint8Array) => void): HandleKey { return keyFromBigint(registerWebHostCallable(callback)); }
export function releaseHostCallable(key: HandleKey): void {
  const value = BigInt.asUintN(64, BigInt(key.high) << 32n | BigInt.asUintN(32, BigInt(key.low)));
  releaseWebHostCallable(value);
  hostRelease?.(key);
}
export function completeHostCall(callId: number, isError: number, content: Uint8Array): void { completeWebHostCall(callId, isError, content); }
export function _seedFunctionRefHandle(_globalIndex: number): [HandleKey, number] { throw new Error("test handle seeding is not implemented yet"); }
export function _seedGenericMediaHandle(): [HandleKey, number] { throw new Error("test handle seeding is not implemented yet"); }
