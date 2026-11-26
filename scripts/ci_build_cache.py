#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import platform
import re
import shlex
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SCHEMA = 1
MANIFEST_NAME = ".pictura-build-cache.json"
PROFILE = (
    "RelWithDebInfo;BUILD_TESTING=ON;CMAKE_DISABLE_PRECOMPILE_HEADERS=ON;"
    "CMAKE_CXX_COMPILER_LAUNCHER=sccache;CARGO_INCREMENTAL=0;"
    "RUSTC_WRAPPER=sccache;CARGO_PROFILE=dev-app;GPU=ON"
)
BUILD_INPUTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/config.toml",
    "CMakeLists.txt",
    "cmake/**/*.cmake",
    "crates/*/Cargo.toml",
    "crates/pictura-app/build.rs",
    "crates/pictura-app/bridge_sources.txt",
    "crates/pictura-app/cpp/tests/CMakeLists.txt",
    ".github/workflows/ci.yml",
    "scripts/ci_build_cache.py",
    "scripts/test-report.sh",
)
VERSION_COMMANDS = (
    ("rustc", "-vV"),
    ("c++", "--version"),
    ("cmake", "--version"),
    ("ninja", "--version"),
    ("mold", "--version"),
    ("ld.lld", "--version"),
    ("qmake6", "-query", "QT_VERSION"),
    ("qmake6", "-query", "QT_INSTALL_PREFIX"),
    ("qmake6", "-query", "QT_INSTALL_HEADERS"),
)


class StaleTree(Exception):
    pass


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def identity_digest(metadata, file_hashes):
    payload = [metadata]
    for name, digest in sorted(file_hashes.items()):
        payload.append(f"{name}\0{digest}")
    return hashlib.sha256("\n".join(payload).encode("utf-8")).hexdigest()


def build_input_hashes(root):
    root = Path(root).resolve()
    paths = set()
    for pattern in BUILD_INPUTS:
        matches = sorted(root.glob(pattern))
        if not matches:
            raise RuntimeError(f"cache identity input has no matches: {pattern}")
        paths.update(path for path in matches if path.is_file())
    return {
        path.relative_to(root).as_posix(): sha256_file(path)
        for path in sorted(paths)
    }


def command_output(command):
    result = subprocess.run(
        command,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return result.stdout.strip()


def make_identity(root, runner_image):
    metadata = [
        f"schema={SCHEMA}",
        f"runner_image={runner_image}",
        f"os={sys.platform}",
        f"arch={platform.machine()}",
        f"profile={PROFILE}",
    ]
    for command in VERSION_COMMANDS:
        metadata.append(f"{shlex.join(command)}={command_output(command)}")
    hashes = build_input_hashes(root)
    digest = identity_digest("\n".join(metadata), hashes)
    details = list(metadata)
    details.extend(f"file:{name}={value}" for name, value in sorted(hashes.items()))
    return digest, "\n".join(details) + "\n"


def safe_build_path(build, relative):
    path = Path(relative)
    if path.is_absolute() or ".." in path.parts:
        raise StaleTree(f"unsafe build path: {relative}")
    build = Path(build).resolve()
    candidate = build / path
    if not candidate.resolve().is_relative_to(build):
        raise StaleTree(f"build path leaves the tree through a symlink: {relative}")
    return candidate


def relative_build_path(build, path):
    build = Path(build).resolve()
    path = Path(path).resolve()
    try:
        return path.relative_to(build).as_posix()
    except ValueError as error:
        raise StaleTree(f"path outside build tree: {path}") from error


def tracked_sources(root):
    root = Path(root).resolve()
    output = subprocess.check_output(["git", "ls-files", "-z"], cwd=root)
    files = {}
    directories = {}
    for raw in output.decode("utf-8").split("\0"):
        if not raw:
            continue
        path = root / raw
        if path.is_symlink() or not path.is_file():
            continue
        files[raw] = sha256_file(path)
        parts = Path(raw).parts
        for index in range(1, len(parts)):
            directory = "/".join(parts[:index])
            directories.setdefault(directory, set()).add(parts[index])
    return {
        "files": files,
        "dirs": {name: sorted(entries) for name, entries in directories.items()},
    }


def cmake_home_directory(build):
    cache = Path(build) / "CMakeCache.txt"
    if not cache.is_file():
        raise StaleTree("CMakeCache.txt is missing")
    for line in cache.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("CMAKE_HOME_DIRECTORY:"):
            return line.split("=", 1)[1]
    raise StaleTree("CMakeCache.txt has no CMAKE_HOME_DIRECTORY")


def configure_values(build):
    cache = Path(build) / "CMakeCache.txt"
    if not cache.is_file():
        raise StaleTree("CMakeCache.txt is missing")
    values = {}
    for line in cache.read_text(encoding="utf-8", errors="replace").splitlines():
        if "=" in line and not line.startswith(("#", "//")):
            key, value = line.split("=", 1)
            values[key.split(":", 1)[0]] = value
    return values


def expected_configure_values(build):
    return {
        "CMAKE_BUILD_TYPE": "RelWithDebInfo",
        "BUILD_TESTING": "ON",
        "CMAKE_DISABLE_PRECOMPILE_HEADERS": "ON",
        "CMAKE_CXX_COMPILER_LAUNCHER": "sccache",
        "FETCHCONTENT_BASE_DIR": str(Path(build).resolve() / "fetchcontent"),
    }


def check_configure_policy(build):
    actual = configure_values(build)
    expected = expected_configure_values(build)
    if any(actual.get(name) != value for name, value in expected.items()):
        raise StaleTree("CMake configure policy differs from the cache identity")
    return expected


def include_roots(build):
    build = Path(build).resolve()
    roots = set()
    commands = build / "compile_commands.json"
    if not commands.is_file():
        raise StaleTree("compile_commands.json is missing")
    try:
        entries = json.loads(commands.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise StaleTree(f"compile_commands.json is invalid: {error}") from error
    for entry in entries:
        directory = Path(entry.get("directory", build))
        tokens = shlex.split(entry.get("command", ""))
        index = 0
        while index < len(tokens):
            token = tokens[index]
            value = None
            if token in ("-I", "-isystem", "-iquote") and index + 1 < len(tokens):
                value = tokens[index + 1]
                index += 1
            elif any(
                token.startswith(prefix) and len(token) > len(prefix)
                for prefix in ("-isystem", "-iquote", "-I")
            ):
                prefix = next(
                    prefix
                    for prefix in ("-isystem", "-iquote", "-I")
                    if token.startswith(prefix)
                )
                value = token[len(prefix):]
            if value is not None:
                path = Path(value)
                if not path.is_absolute():
                    path = directory / path
                path = path.resolve()
                if path.is_relative_to(build) and (
                    path.name == "include"
                    or any(part.endswith("_autogen") for part in path.parts)
                ):
                    roots.add(path)
            index += 1
    for path in build.rglob("*_autogen"):
        if path.is_dir():
            roots.add(path.resolve())
    cargo_build = build / "cargo" / "build"
    if cargo_build.is_dir():
        for output in cargo_build.glob("pictura_app-*/out"):
            if output.is_dir():
                for path in output.rglob("include"):
                    if path.is_dir():
                        roots.add(path.resolve())
    return sorted(roots)


def ctest_executables(root, build, ctest_json):
    data = json.loads(Path(ctest_json).read_text(encoding="utf-8"))
    executables = set()
    for test in data.get("tests", []):
        command = test.get("command") or []
        if not command:
            continue
        executable = Path(command[0])
        if not executable.is_absolute():
            candidate = Path(build) / executable
            executable = candidate if candidate.exists() else Path(root) / executable
        executable = executable.resolve()
        if not executable.is_relative_to(Path(build).resolve()):
            continue
        if "-o" in command and (
            not executable.is_file() or not os.access(executable, os.X_OK)
        ):
            raise StaleTree(f"registered executable is missing: {executable}")
        if executable.is_file() and os.access(executable, os.X_OK):
            executables.add(executable.relative_to(Path(build).resolve()).as_posix())
    return sorted(executables)


def make_manifest(root, build, fingerprint, cache_key, source_commit, ctest_json):
    root = Path(root).resolve()
    build = Path(build).resolve()
    if cmake_home_directory(build) != str(root):
        raise StaleTree("build tree belongs to a different source directory")
    configure = check_configure_policy(build)

    required_files = {
        "CMakeCache.txt",
        "build.ninja",
        "rules.ninja",
        ".ninja_log",
        ".ninja_deps",
        "compile_commands.json",
        "pictura",
    }
    required_dirs = {"cargo"}
    executables = {"pictura"}
    generated_files = {}

    shells = list(build.glob("libpictura_shell.*")) + list(build.glob("pictura_shell.*"))
    shells = [path for path in shells if path.is_file()]
    if not shells:
        raise StaleTree("pictura_shell library is missing")
    required_files.update(relative_build_path(build, path) for path in shells)

    for name in required_files | executables:
        path = safe_build_path(build, name)
        if not path.is_file():
            raise StaleTree(f"required build output is missing: {name}")
    for name in executables:
        if not os.access(safe_build_path(build, name), os.X_OK):
            raise StaleTree(f"required executable is not executable: {name}")

    cargo_build = build / "cargo" / "build"
    cargo_outputs = sorted(cargo_build.glob("pictura_app-*/output"))
    if not cargo_build.is_dir():
        raise StaleTree("Corrosion Cargo build directory is missing")
    if not cargo_outputs:
        raise StaleTree("Corrosion pictura-app build-script output is missing")
    for path in cargo_outputs:
        name = relative_build_path(build, path)
        required_files.add(name)
        generated_files[name] = sha256_file(path)

    fetch_deps = build / "fetchcontent" / "_deps"
    if fetch_deps.is_dir():
        required_dirs.add("fetchcontent/_deps")
        for path in sorted(fetch_deps.glob("*-src/CMakeLists.txt")):
            required_files.add(relative_build_path(build, path))

    roots = include_roots(build)
    if not roots:
        raise StaleTree("no generated include or autogen roots were found")
    for directory in roots:
        required_dirs.add(relative_build_path(build, directory))
        for path in sorted(directory.rglob("*")):
            if path.is_file() and not path.is_symlink():
                name = relative_build_path(build, path)
                generated_files[name] = sha256_file(path)
        if directory.name == "include" and directory.parent.name == "pictura_app":
            initializer = directory.parent / "initializers.o"
            if not initializer.is_file():
                raise StaleTree(f"cxx-qt initializer is missing: {initializer}")
            name = relative_build_path(build, initializer)
            required_files.add(name)
            generated_files[name] = sha256_file(initializer)

    executables.update(ctest_executables(root, build, ctest_json))
    for name in executables:
        path = safe_build_path(build, name)
        if not path.is_file() or not os.access(path, os.X_OK):
            raise StaleTree(f"registered executable is missing: {name}")

    for name in required_dirs:
        if not safe_build_path(build, name).is_dir():
            raise StaleTree(f"required build directory is missing: {name}")
    for name in required_files:
        if not safe_build_path(build, name).is_file():
            raise StaleTree(f"required build file is missing: {name}")

    saved_at = time.time_ns()
    output_times = [
        safe_build_path(build, name).stat().st_mtime_ns
        for name in required_files | executables | set(generated_files)
    ]
    source_epoch = min([saved_at, *output_times]) - 1_000_000_000
    sources = tracked_sources(root)
    return {
        "schema": SCHEMA,
        "fingerprint": fingerprint,
        "cache_key": cache_key,
        "source_commit": source_commit,
        "saved_at_ns": saved_at,
        "source_epoch_ns": source_epoch,
        "source_home": str(root),
        "configure": configure,
        "source_files": sources["files"],
        "source_dirs": sources["dirs"],
        "required_files": sorted(required_files),
        "required_dirs": sorted(required_dirs),
        "executables": sorted(executables),
        "generated_files": generated_files,
    }


def validate_manifest(root, build, manifest, fingerprint):
    if manifest.get("schema") != SCHEMA:
        raise StaleTree("unsupported cache manifest schema")
    if manifest.get("fingerprint") != fingerprint:
        raise StaleTree("cache fingerprint differs from the current build identity")
    if manifest.get("source_home") != str(Path(root).resolve()):
        raise StaleTree("cache source directory differs from the current checkout")
    if cmake_home_directory(build) != str(Path(root).resolve()):
        raise StaleTree("CMakeCache source directory differs from the current checkout")
    if manifest.get("configure") != check_configure_policy(build):
        raise StaleTree("cache configure policy differs from the current build")
    for name in manifest.get("required_dirs", []):
        if not safe_build_path(build, name).is_dir():
            raise StaleTree(f"required build directory is missing: {name}")
    for name in manifest.get("required_files", []):
        if not safe_build_path(build, name).is_file():
            raise StaleTree(f"required build file is missing: {name}")
    for name in manifest.get("executables", []):
        path = safe_build_path(build, name)
        if not path.is_file() or not os.access(path, os.X_OK):
            raise StaleTree(f"registered executable is missing: {name}")
    for name, digest in manifest.get("generated_files", {}).items():
        path = safe_build_path(build, name)
        if not path.is_file():
            raise StaleTree(f"generated output is missing: {name}")
        if sha256_file(path) != digest:
            raise StaleTree(f"generated output differs from its saved digest: {name}")


def normalize_source_times(root, manifest):
    root = Path(root).resolve()
    current = tracked_sources(root)
    old_files = manifest.get("source_files", {})
    old_dirs = manifest.get("source_dirs", {})
    epoch = int(manifest["source_epoch_ns"])
    now = time.time_ns()
    for name, digest in current["files"].items():
        modified = epoch if old_files.get(name) == digest else now
        os.utime(root / name, ns=(modified, modified), follow_symlinks=False)
    directories = sorted(
        set(old_dirs) | set(current["dirs"]),
        key=lambda name: (name.count("/"), name),
        reverse=True,
    )
    for name in directories:
        path = root / name
        if not path.is_dir():
            continue
        modified = epoch if old_dirs.get(name) == current["dirs"].get(name) else now
        os.utime(path, ns=(modified, modified), follow_symlinks=False)


def restore_tree(root, build, fingerprint):
    build = Path(build).resolve()
    manifest_path = build / MANIFEST_NAME
    if not manifest_path.is_file():
        raise StaleTree(f"cache manifest is missing: {manifest_path}")
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise StaleTree(f"cache manifest is invalid: {error}") from error
    validate_manifest(root, build, manifest, fingerprint)
    normalize_source_times(root, manifest)
    return manifest


def ninja_edges(log_text):
    text = re.sub(r"\x1b\[[0-9;]*m", "", log_text)
    matches = re.findall(r"^\[(\d+)/(\d+)\]", text, flags=re.MULTILINE)
    if matches:
        return int(matches[-1][1])
    if "ninja: no work to do." in text:
        return 0
    return None


def build_summary(log_text):
    # The shared Qt runner invokes Ninja once for pictura and once for the
    # Qt Test build tail. A no-op in either invocation contributes zero edges.
    stages = re.findall(
        r"^== (?:pictura build|qt build tail) ==[^\n]*\n(.*?)(?=^== |\Z)",
        log_text,
        flags=re.MULTILINE | re.DOTALL,
    )
    edge_counts = [ninja_edges(stage) for stage in stages]
    durations = re.findall(
        r"^timing: (?:pictura_build|qt_build_tail)=([\d.]+)s$", log_text, re.MULTILINE
    )
    return {
        "ninja_edges": (
            sum(edge_counts)
            if len(edge_counts) == 2 and all(count is not None for count in edge_counts)
            else None
        ),
        "duration_seconds": (
            round(sum(map(float, durations)), 3) if len(durations) == 2 else None
        ),
    }


def write_manifest_command(args):
    manifest = make_manifest(
        args.root,
        args.build_dir,
        args.fingerprint,
        args.cache_key,
        args.source_commit,
        args.ctest_json,
    )
    output = Path(args.output) if args.output else Path(args.build_dir) / MANIFEST_NAME
    output.write_text(
        json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    print(f"cache manifest: {output}")
    return 0


def restore_command(args):
    try:
        restore_tree(args.root, args.build_dir, args.fingerprint)
    except StaleTree as error:
        print(f"stale build cache: {error}", file=sys.stderr)
        return 2
    print("build cache: valid")
    return 0


def identity_command(args):
    digest, details = make_identity(args.root, args.runner_image)
    Path(args.fingerprint_file).write_text(details, encoding="utf-8")
    prefix = (
        f"qt-headless-build-v1-{sys.platform}-{platform.machine()}-{digest}-"
    )
    output = f"prefix={prefix}\nfingerprint={digest}\n"
    if args.github_output:
        with Path(args.github_output).open("a", encoding="utf-8") as handle:
            handle.write(output)
    print(output, end="")
    return 0


def build_summary_command(args):
    summary = build_summary(Path(args.log).read_text(encoding="utf-8"))
    print(json.dumps(summary))
    if args.github_output:
        with Path(args.github_output).open("a", encoding="utf-8") as handle:
            for key, value in summary.items():
                handle.write(f"{key}={value if value is not None else 'unavailable'}\n")
    return 0


def make_fixture(base):
    root = base / "repo"
    build = base / "build"
    root.mkdir()
    (root / "src").mkdir()
    (root / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
    (root / "CMakeLists.txt").write_text("project(fixture)\n", encoding="utf-8")
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    subprocess.run(["git", "add", "."], cwd=root, check=True)
    build.mkdir()
    (build / "CMakeCache.txt").write_text(
        f"CMAKE_HOME_DIRECTORY:INTERNAL={root}\n"
        "CMAKE_BUILD_TYPE:STRING=RelWithDebInfo\n"
        "BUILD_TESTING:BOOL=ON\n"
        "CMAKE_DISABLE_PRECOMPILE_HEADERS:BOOL=ON\n"
        "CMAKE_CXX_COMPILER_LAUNCHER:STRING=sccache\n"
        f"FETCHCONTENT_BASE_DIR:PATH={build / 'fetchcontent'}\n",
        encoding="utf-8",
    )
    for name in ("build.ninja", "rules.ninja", ".ninja_log", ".ninja_deps"):
        (build / name).write_text("fixture\n", encoding="utf-8")
    include = build / "cargo" / "build" / "pictura-app-fixture" / "out" / "crates" / "pictura_app" / "include"
    include.mkdir(parents=True)
    (include / "generated.h").write_text("generated\n", encoding="utf-8")
    (include.parent / "initializers.o").write_bytes(b"object\n")
    (build / "cargo" / "build" / "pictura-app-fixture" / "output").write_text(
        "cargo:rustc-link-lib=fixture\n", encoding="utf-8"
    )
    autogen = build / "crates" / "pictura-app" / "cpp" / "fixture_autogen"
    autogen.mkdir(parents=True)
    (autogen / "moc_fixture.cpp").write_text("moc\n", encoding="utf-8")
    fetch = build / "fetchcontent" / "_deps" / "cxxqt-src"
    fetch.mkdir(parents=True)
    (fetch / "CMakeLists.txt").write_text("project(fixture)\n", encoding="utf-8")
    (build / "pictura").write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    (build / "pictura").chmod(0o755)
    (build / "libpictura_shell.so").write_bytes(b"shell\n")
    (build / "tst_fixture").write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    (build / "tst_fixture").chmod(0o755)
    (build / "compile_commands.json").write_text(
        json.dumps([
            {
                "directory": str(root),
                "command": f"c++ -I{include} -c src/main.rs",
            }
        ]),
        encoding="utf-8",
    )
    ctest_json = base / "ctest.json"
    ctest_json.write_text(
        json.dumps({
            "tests": [
                {
                    "name": "tst_fixture",
                    "command": [
                        str(build / "tst_fixture"),
                        "-o",
                        str(build / "qt-test-results" / "tst_fixture.xml,junitxml"),
                    ],
                }
            ]
        }),
        encoding="utf-8",
    )
    return root, build, ctest_json


def fixture_manifest(base):
    root, build, ctest_json = make_fixture(base)
    manifest = make_manifest(
        root, build, "fixture", "fixture-key", "fixture-commit", ctest_json
    )
    (build / MANIFEST_NAME).write_text(json.dumps(manifest), encoding="utf-8")
    return root, build, manifest


def self_test_missing_generated_rejected():
    with tempfile.TemporaryDirectory() as temporary:
        root, build, _ = fixture_manifest(Path(temporary))
        (build / "cargo" / "build" / "pictura-app-fixture" / "out" / "crates" / "pictura_app" / "include" / "generated.h").unlink()
        try:
            restore_tree(root, build, "fixture")
        except StaleTree as error:
            assert "generated output is missing" in str(error), error
        else:
            raise AssertionError("missing generated output was accepted")


def self_test_changed_generated_rejected():
    with tempfile.TemporaryDirectory() as temporary:
        root, build, _ = fixture_manifest(Path(temporary))
        (build / "cargo" / "build" / "pictura-app-fixture" / "out" / "crates" / "pictura_app" / "include" / "generated.h").write_text("changed\n", encoding="utf-8")
        try:
            restore_tree(root, build, "fixture")
        except StaleTree as error:
            assert "differs from its saved digest" in str(error), error
        else:
            raise AssertionError("changed generated output was accepted")


def self_test_missing_qt_executable_rejected():
    with tempfile.TemporaryDirectory() as temporary:
        root, build, _ = fixture_manifest(Path(temporary))
        (build / "tst_fixture").unlink()
        try:
            restore_tree(root, build, "fixture")
        except StaleTree as error:
            assert "registered executable is missing" in str(error), error
        else:
            raise AssertionError("a missing Qt Test executable was accepted")


def self_test_identity_mismatch_rejected():
    with tempfile.TemporaryDirectory() as temporary:
        root, build, _ = fixture_manifest(Path(temporary))
        before = (root / "src" / "main.rs").stat().st_mtime_ns
        try:
            restore_tree(root, build, "different")
        except StaleTree as error:
            assert "fingerprint differs" in str(error), error
        else:
            raise AssertionError("different identity was accepted")
        assert (root / "src" / "main.rs").stat().st_mtime_ns == before


def self_test_identity_changes_with_build_input():
    first = identity_digest("metadata", {"Cargo.lock": "first"})
    second = identity_digest("metadata", {"Cargo.lock": "second"})
    assert first != second
    assert first == identity_digest("metadata", {"Cargo.lock": "first"})


def self_test_source_times_normalized():
    with tempfile.TemporaryDirectory() as temporary:
        root, build, manifest = fixture_manifest(Path(temporary))
        (root / "src" / "main.rs").write_text("fn main() { changed(); }\n", encoding="utf-8")
        (root / "src" / "new.rs").write_text("pub fn new() {}\n", encoding="utf-8")
        subprocess.run(["git", "add", "src/new.rs"], cwd=root, check=True)
        now = time.time_ns()
        for name in ("CMakeLists.txt", "src/main.rs", "src/new.rs", "src"):
            os.utime(root / name, ns=(now, now))
        restore_tree(root, build, "fixture")
        epoch = manifest["source_epoch_ns"]
        assert epoch < (build / "pictura").stat().st_mtime_ns
        assert (root / "CMakeLists.txt").stat().st_mtime_ns == epoch
        assert (root / "src" / "main.rs").stat().st_mtime_ns > epoch
        assert (root / "src" / "new.rs").stat().st_mtime_ns > epoch
        assert (root / "src").stat().st_mtime_ns > epoch


def self_test_configure_policy_rejected():
    with tempfile.TemporaryDirectory() as temporary:
        root, build, _ = fixture_manifest(Path(temporary))
        cache = build / "CMakeCache.txt"
        cache.write_text(
            cache.read_text(encoding="utf-8").replace(
                "CMAKE_DISABLE_PRECOMPILE_HEADERS:BOOL=ON",
                "CMAKE_DISABLE_PRECOMPILE_HEADERS:BOOL=OFF",
            ),
            encoding="utf-8",
        )
        try:
            restore_tree(root, build, "fixture")
        except StaleTree as error:
            assert "configure policy differs" in str(error), error
        else:
            raise AssertionError("a PCH-enabled configure policy was accepted")


def self_test_build_summary():
    assert ninja_edges("[1/3] compile\n[3/3] link\n") == 3
    assert ninja_edges("ninja: no work to do.\n") == 0
    assert ninja_edges("configure only\n") is None
    log = (
        "== pictura build ==\n[1/3] compile\n[3/3] link\n"
        "timing: pictura_build=1.500s\n"
        "== qt build tail ==\nninja: no work to do.\n"
        "timing: qt_build_tail=0.250s\n"
    )
    assert build_summary(log) == {"ninja_edges": 3, "duration_seconds": 1.75}


def run_self_test():
    tests = (
        self_test_missing_generated_rejected,
        self_test_missing_qt_executable_rejected,
        self_test_changed_generated_rejected,
        self_test_identity_mismatch_rejected,
        self_test_identity_changes_with_build_input,
        self_test_source_times_normalized,
        self_test_configure_policy_rejected,
        self_test_build_summary,
    )
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print("ci_build_cache.py: self-test OK")
    return 0


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    commands = parser.add_subparsers(dest="command")

    identity = commands.add_parser("identity")
    identity.add_argument("--root", default=".")
    identity.add_argument("--runner-image", required=True)
    identity.add_argument("--fingerprint-file", required=True)
    identity.add_argument("--github-output")

    manifest = commands.add_parser("write-manifest")
    manifest.add_argument("--root", default=".")
    manifest.add_argument("--build-dir", default="build")
    manifest.add_argument("--fingerprint", required=True)
    manifest.add_argument("--cache-key", required=True)
    manifest.add_argument("--source-commit", required=True)
    manifest.add_argument("--ctest-json", required=True)
    manifest.add_argument("--output")

    restore = commands.add_parser("restore")
    restore.add_argument("--root", default=".")
    restore.add_argument("--build-dir", default="build")
    restore.add_argument("--fingerprint", required=True)

    summary = commands.add_parser("build-summary")
    summary.add_argument("--log", required=True)
    summary.add_argument("--github-output")
    return parser.parse_args()


def main():
    args = parse_args()
    if args.self_test:
        return run_self_test()
    if args.command == "identity":
        return identity_command(args)
    if args.command == "write-manifest":
        return write_manifest_command(args)
    if args.command == "restore":
        return restore_command(args)
    if args.command == "build-summary":
        return build_summary_command(args)
    raise SystemExit("a command is required")


if __name__ == "__main__":
    raise SystemExit(main())
