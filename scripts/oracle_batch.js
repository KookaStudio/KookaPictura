#!/usr/bin/env node

const fs = require('fs');
const vm = require('vm');

const MAX_FIELD = 16 * 1024 * 1024;

class RequestExit extends Error {
  constructor(code) {
    super('requested exit');
    this.code = code;
  }
}

function runRequest(request, stdout, stderr) {
  if (!request.args.length) throw new Error('oracle request has no -e source');
  const oldCwd = process.cwd();
  const oldEnv = request.env.map(([key]) => [key, process.env[key]]);
  let code = 0;
  try {
    if (request.cwd !== null) process.chdir(request.cwd);
    for (const [key, value] of request.env) {
      if (value === null) delete process.env[key];
      else process.env[key] = value;
    }
    const requestProcess = Object.create(process);
    requestProcess.argv = [process.execPath, ...request.args.slice(2)];
    requestProcess.exitCode = 0;
    requestProcess.exit = (value) => {
      throw new RequestExit(value === undefined ? 0 : value);
    };
    const context = {
      require,
      process: requestProcess,
      console: {
        log: (...values) => stdout.push(Buffer.from(values.join(' ') + '\n')),
        info: (...values) => stdout.push(Buffer.from(values.join(' ') + '\n')),
        warn: (...values) => stderr.push(Buffer.from(values.join(' ') + '\n')),
        error: (...values) => stderr.push(Buffer.from(values.join(' ') + '\n')),
      },
      Buffer,
      Uint8Array,
      Uint16Array,
      Uint32Array,
      Int8Array,
      Int16Array,
      Int32Array,
      Float32Array,
      Float64Array,
      ArrayBuffer,
      DataView,
      setTimeout,
      clearTimeout,
      setInterval,
      clearInterval,
      module: {exports: {}},
      exports: {},
    };
    vm.runInNewContext(request.args[1], context, {filename: 'oracle-eval'});
    code = Number.isInteger(requestProcess.exitCode) ? requestProcess.exitCode : 0;
  } catch (error) {
    if (error instanceof RequestExit) {
      code = Number.isInteger(error.code) ? error.code : error.code === undefined ? 0 : 1;
    } else {
      stderr.push(Buffer.from((error && error.stack ? error.stack : String(error)) + '\n'));
      code = 1;
    }
  } finally {
    process.chdir(oldCwd);
    for (const [key, value] of oldEnv) {
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    }
  }
  return code;
}

function writeAll(fd, data) {
  let offset = 0;
  while (offset < data.length) {
    offset += fs.writeSync(fd, data, offset, data.length - offset);
  }
}

function writeReply(code, stdout, stderr) {
  const header = Buffer.alloc(20);
  header.writeInt32LE(code, 0);
  header.writeBigUInt64LE(BigInt(stdout.length), 4);
  header.writeBigUInt64LE(BigInt(stderr.length), 12);
  writeAll(1, header);
  writeAll(1, stdout);
  writeAll(1, stderr);
}

function main() {
  let buffered = Buffer.alloc(0);
  function readExactInput(size) {
    while (buffered.length < size) {
      const chunk = Buffer.alloc(65536);
      const read = fs.readSync(0, chunk, 0, chunk.length, null);
      if (!read) return null;
      buffered = Buffer.concat([buffered, chunk.subarray(0, read)]);
    }
    const result = buffered.subarray(0, size);
    buffered = buffered.subarray(size);
    return result;
  }
  while (true) {
    const cwdSizeData = readExactInput(8);
    if (!cwdSizeData) return 0;
    const cwdSize = cwdSizeData.readBigUInt64LE(0);
    const cwd = cwdSize === (1n << 64n) - 1n ? null : (() => {
      if (cwdSize > BigInt(MAX_FIELD)) throw new Error('oracle request field is too large');
      return readExactInput(Number(cwdSize)).toString();
    })();
    const countData = readExactInput(8);
    const count = Number(countData.readBigUInt64LE(0));
    const args = [];
    for (let index = 0; index < count; index += 1) {
      const sizeData = readExactInput(8);
      const size = Number(sizeData.readBigUInt64LE(0));
      if (size > MAX_FIELD) throw new Error('oracle request field is too large');
      args.push(readExactInput(size).toString());
    }
    const envCountData = readExactInput(8);
    const envCount = Number(envCountData.readBigUInt64LE(0));
    const env = [];
    for (let index = 0; index < envCount; index += 1) {
      const keySizeData = readExactInput(8);
      const keySize = Number(keySizeData.readBigUInt64LE(0));
      if (keySize > MAX_FIELD) throw new Error('oracle request field is too large');
      const key = readExactInput(keySize).toString();
      const present = readExactInput(1);
      let value = null;
      if (present[0] === 1) {
        const valueSizeData = readExactInput(8);
        const valueSize = Number(valueSizeData.readBigUInt64LE(0));
        if (valueSize > MAX_FIELD) throw new Error('oracle request field is too large');
        value = readExactInput(valueSize).toString();
      }
      env.push([key, value]);
    }
    const stdout = [];
    const stderr = [];
    const code = runRequest({cwd, args, env}, stdout, stderr);
    writeReply(code, Buffer.concat(stdout), Buffer.concat(stderr));
  }
}

try {
  main();
} catch (error) {
  process.stderr.write((error && error.stack ? error.stack : String(error)) + '\n');
  process.exitCode = 1;
}
