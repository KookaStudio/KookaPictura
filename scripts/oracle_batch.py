#!/usr/bin/env python3

import io
import os
import runpy
import struct
import sys
import traceback

MAX_FIELD = 16 * 1024 * 1024
MODULES = {}


class Capture(io.TextIOBase):
    def __init__(self):
        self.buffer = io.BytesIO()

    @property
    def encoding(self):
        return "utf-8"

    def write(self, value):
        data = value.encode("utf-8", "surrogateescape") if isinstance(value, str) else bytes(value)
        self.buffer.write(data)
        return len(value)

    def flush(self):
        return None

    def isatty(self):
        return False

    def getvalue(self):
        return self.buffer.getvalue()


def read_exact(stream, size):
    chunks = bytearray()
    while len(chunks) < size:
        chunk = stream.read(size - len(chunks))
        if not chunk:
            return None
        chunks.extend(chunk)
    return bytes(chunks)


def read_u64(stream):
    data = read_exact(stream, 8)
    if data is None:
        raise EOFError
    return struct.unpack("<Q", data)[0]


def read_field(stream):
    size = read_u64(stream)
    if size > MAX_FIELD:
        raise ValueError("oracle request field is too large")
    data = read_exact(stream, size)
    if data is None:
        raise EOFError
    return data


def read_request(stream):
    cwd_size = read_u64(stream)
    cwd = None if cwd_size == (1 << 64) - 1 else os.fsdecode(read_exact(stream, cwd_size))
    args = [os.fsdecode(read_field(stream)) for _ in range(read_u64(stream))]
    env = []
    for _ in range(read_u64(stream)):
        key = os.fsdecode(read_field(stream))
        present = read_exact(stream, 1)
        if present is None:
            raise EOFError
        value = os.fsdecode(read_field(stream)) if present == b"\x01" else None
        env.append((key, value))
    return cwd, args, env


def load_module(path):
    if path not in MODULES:
        MODULES[path] = runpy.run_path(path, run_name="__oracle_batch__")
    return MODULES[path]


def exit_code(value):
    if value is None:
        return 0
    if isinstance(value, int):
        return value
    return 1


def run_request(cwd, args, env, stdout, stderr):
    if not args:
        raise ValueError("oracle request has no script or -c source")
    script = None if args[0] == "-c" else os.path.abspath(args[0])
    old_argv = sys.argv
    old_cwd = os.getcwd()
    old_env = {key: os.environ.get(key) for key, _ in env}
    code = 0
    try:
        if cwd is not None:
            os.chdir(cwd)
        for key, value in env:
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
        sys.argv = ["-c", *args[2:]] if script is None else [script, *args[1:]]
        if script is None:
            exec(compile(args[1], "<oracle>", "exec"), {"__name__": "__main__"})
        else:
            result = load_module(script)["main"]()
            code = exit_code(result)
    except SystemExit as error:
        code = exit_code(error.code)
        if error.code is not None and not isinstance(error.code, int):
            print(error.code, file=stderr)
    except BaseException:
        traceback.print_exc(file=stderr)
        code = 1
    finally:
        sys.argv = old_argv
        os.chdir(old_cwd)
        for key, value in old_env.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
    return code


def write_reply(stream, code, stdout, stderr):
    stream.write(struct.pack("<iQQ", code, len(stdout), len(stderr)))
    stream.write(stdout)
    stream.write(stderr)
    stream.flush()


def main():
    source = sys.stdin.buffer
    sink = sys.stdout.buffer
    while True:
        try:
            cwd, args, env = read_request(source)
        except EOFError:
            return 0
        except Exception as error:
            write_reply(sink, 1, b"", f"{error}\n".encode())
            return 1
        stdout = Capture()
        stderr = Capture()
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        code = 1
        try:
            sys.stdout = stdout
            sys.stderr = stderr
            code = run_request(cwd, args, env, stdout, stderr)
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr
        write_reply(sink, code, stdout.getvalue(), stderr.getvalue())


if __name__ == "__main__":
    sys.exit(main())
