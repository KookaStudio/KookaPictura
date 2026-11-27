#!/usr/bin/env python3
"""Audit an existing normal Cargo build report or mixed CMake log; never initiate a Cargo build.

Usage: check-view-oracle.py --profile release|dev-app <cargo-json-report-or-cmake-log>
       check-view-oracle.py --self-check
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import tempfile


def cargo_records(report: str) -> list[dict]:
    records = []
    for line in report.splitlines():
        if not line.lstrip().startswith("{"):
            continue
        record = json.loads(line)
        if not isinstance(record, dict):
            raise ValueError("expected Cargo JSON objects")
        records.append(record)
    if not records:
        raise ValueError("expected Cargo JSON objects")
    return records


def renderer_artifact(report: str, profile: str) -> Path:
    records = cargo_records(report)
    finished = [record for record in records if record.get("reason") == "build-finished"]
    if len(finished) != 1 or finished[0].get("success") is not True:
        raise ValueError("report has no unique successful build-finished record")
    artifacts = [
        record for record in records
        if record.get("reason") == "compiler-artifact"
        and isinstance(record.get("target"), dict)
        and record["target"].get("name") == "pictura_render"
    ]
    if len(artifacts) != 1:
        raise ValueError("expected exactly one pictura_render compiler artifact")
    artifact = artifacts[0]
    features = artifact.get("features")
    if not isinstance(features, list) or any(not isinstance(item, str) for item in features):
        raise ValueError("renderer artifact has no valid feature list")
    if "test-oracle" in features:
        raise ValueError("renderer artifact enables test-oracle")
    build_profile = artifact.get("profile")
    if not isinstance(build_profile, dict) or build_profile.get("test") is not False:
        raise ValueError("renderer artifact is not a normal library build")
    filenames = artifact.get("filenames")
    if not isinstance(filenames, list) or any(not isinstance(item, str) for item in filenames):
        raise ValueError("renderer artifact has no valid filenames")
    libraries = [Path(item) for item in filenames if item.endswith(".rlib")]
    if len(libraries) != 1:
        raise ValueError("expected exactly one renderer rlib")
    library = libraries[0].resolve()
    if library.parent.name != "deps" or library.parent.parent.name != profile:
        raise ValueError(f"renderer artifact is not from the {profile} profile")
    if not library.is_file():
        raise ValueError(f"renderer artifact does not exist: {library}")
    return library


def dependency_dirs(report: str, library: Path) -> list[Path]:
    # Cargo with --target puts procedural macros in the host profile directory.
    directories = {library.parent}
    for record in cargo_records(report):
        if record.get("reason") != "compiler-artifact":
            continue
        target = record.get("target")
        if isinstance(target, dict) and target.get("kind") == ["proc-macro"]:
            directories.update(Path(filename).resolve().parent for filename in record["filenames"])
    return [library.parent, *sorted(directories - {library.parent})]


def oracle_imports_are_missing(result: subprocess.CompletedProcess[str]) -> bool:
    if result.returncode == 0:
        return False
    try:
        diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.strip()]
    except json.JSONDecodeError:
        return False
    if any(not isinstance(item, dict) for item in diagnostics):
        return False
    errors = [item for item in diagnostics if item.get("level") == "error"]
    imports = [
        item for item in errors
        if isinstance(item.get("code"), dict) and item["code"].get("code") == "E0432"
    ]
    unexpected = [
        item for item in errors
        if item not in imports
        and not (
            isinstance(item.get("message"), str)
            and item["message"].startswith("aborting due to")
        )
    ]
    messages = " ".join(
        item["message"] for item in imports if isinstance(item.get("message"), str)
    )
    return not unexpected and all(name in messages for name in ("Planes", "ViewPyramid"))


def check_profile(report: Path, profile: str, rustc: str = "rustc") -> None:
    report_text = report.read_text(encoding="utf-8")
    library = renderer_artifact(report_text, profile)
    with tempfile.TemporaryDirectory(prefix="pictura-view-oracle-") as directory:
        work = Path(directory)
        source = work / "probe.rs"
        command = [
            rustc, "--edition=2021", "--crate-type=lib", "--emit=metadata",
            "--crate-name=view_oracle_probe", "--error-format=json",
            "--extern", f"pictura_render={library}",
        ]
        for deps in dependency_dirs(report_text, library):
            command.extend(["-L", f"dependency={deps}"])
        command.extend(["--out-dir", str(work), str(source)])
        source.write_text(
            "use pictura_render::{PyramidLevel, SMALLEST_SIDE, TILE};\n"
            "pub fn shared(level: &PyramidLevel) -> (u32, u32, u32, i32) {\n"
            "    (level.width(), level.height(), SMALLEST_SIDE, TILE)\n}\n",
            encoding="utf-8",
        )
        positive = subprocess.run(command, capture_output=True, text=True, check=False)
        if positive.returncode:
            raise ValueError(f"shared-export probe failed: {positive.stderr}")
        source.write_text(
            "use pictura_render::{Planes, ViewPyramid};\n",
            encoding="utf-8",
        )
        negative = subprocess.run(command, capture_output=True, text=True, check=False)
        if not oracle_imports_are_missing(negative):
            raise ValueError(f"oracle imports did not fail as expected: {negative.stderr}")
    print(f"view-oracle: {profile}: feature off, shared exports usable, oracle imports absent")


def self_check() -> None:
    with tempfile.TemporaryDirectory(prefix="pictura-view-oracle-check-") as directory:
        artifacts = {}
        libraries = {}
        for profile in ("release", "dev-app"):
            library = Path(directory) / profile / "deps/libpictura_render.rlib"
            library.parent.mkdir(parents=True)
            library.touch()
            libraries[profile] = library
            artifacts[profile] = {
                "reason": "compiler-artifact", "target": {"name": "pictura_render"},
                "features": [], "profile": {"test": False}, "filenames": [str(library)],
            }
        finished = {"reason": "build-finished", "success": True}

        def report(*records: dict) -> str:
            return "\n".join(json.dumps(record) for record in records)

        for profile, library in libraries.items():
            assert renderer_artifact(report(artifacts[profile], finished), profile) == library
        mixed = (
            "[1/2] Building pictura_render\n"
            + report(artifacts["dev-app"], finished)
            + "\n[2/2] Linking pictura_app\n"
        )
        assert renderer_artifact(mixed, "dev-app") == libraries["dev-app"]

        host_deps = libraries["dev-app"].parent
        target_library = Path(directory) / "x86_64-unknown-linux-gnu/dev-app/deps/libpictura_render.rlib"
        target_library.parent.mkdir(parents=True)
        target_library.touch()
        proc_macro = {
            "reason": "compiler-artifact", "target": {"kind": ["proc-macro"]},
            "filenames": [str(host_deps / "libthiserror_impl.so")],
        }
        target_report = report(
            dict(artifacts["dev-app"], filenames=[str(target_library)]), proc_macro, finished,
        )
        assert renderer_artifact(target_report, "dev-app") == target_library
        assert dependency_dirs(target_report, target_library) == [target_library.parent, host_deps]
        assert dependency_dirs(report(artifacts["dev-app"], proc_macro, finished), libraries["dev-app"]) == [host_deps]

        artifact = artifacts["release"]
        invalid = [
            "not JSON", '{"reason":', report(finished), report(artifact),
            report(artifact, artifact, finished),
            report(dict(artifact, target=None), finished),
            report(dict(artifact, features=["test-oracle"]), finished),
            report(dict(artifact, features=None), finished),
            report(dict(artifact, profile={"test": True}), finished),
            report(dict(artifact, profile=None), finished),
            report(dict(artifact, filenames=[]), finished),
            report(dict(artifact, filenames=[None]), finished),
            report(artifact, dict(finished, success=False)),
            mixed + '{"reason":',
        ]
        for value in invalid:
            try:
                renderer_artifact(value, "release")
            except (ValueError, TypeError):
                pass
            else:
                raise AssertionError(f"accepted an invalid report: {value}")
        for profile, other in (("release", "dev-app"), ("dev-app", "release")):
            try:
                renderer_artifact(report(artifacts[profile], finished), other)
            except ValueError:
                pass
            else:
                raise AssertionError("accepted the wrong profile")
        for profile, library in libraries.items():
            library.unlink()
            try:
                renderer_artifact(report(artifacts[profile], finished), profile)
            except ValueError:
                pass
            else:
                raise AssertionError("accepted a missing artifact")

    def diagnostic(code: object, message: object) -> str:
        return json.dumps({"level": "error", "code": {"code": code}, "message": message})

    missing = diagnostic("E0432", "unresolved imports pictura_render::Planes, pictura_render::ViewPyramid")
    assert oracle_imports_are_missing(subprocess.CompletedProcess([], 1, "", missing))
    malformed_code = json.dumps({"level": "error", "code": "E0432", "message": missing})
    for result in [
        subprocess.CompletedProcess([], 0, "", missing),
        subprocess.CompletedProcess([], 1, "", "compiler unavailable"),
        subprocess.CompletedProcess([], 1, "", diagnostic("E0463", "cannot find crate")),
        subprocess.CompletedProcess([], 1, "", diagnostic("E0432", "unresolved import Planes")),
        subprocess.CompletedProcess([], 1, "", diagnostic("E0432", None)),
        subprocess.CompletedProcess([], 1, "", malformed_code),
        subprocess.CompletedProcess([], 1, "", missing + "\n" + diagnostic("E0463", "cannot find crate")),
    ]:
        assert not oracle_imports_are_missing(result)
    print("view-oracle: self-check OK")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path, nargs="?")
    parser.add_argument("--profile", choices=("release", "dev-app"))
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    try:
        if args.self_check:
            self_check()
        elif args.report is None or args.profile is None:
            parser.error("report and --profile are required without --self-check")
        else:
            check_profile(args.report, args.profile, args.rustc)
    except (OSError, ValueError, TypeError, KeyError) as error:
        parser.exit(1, f"view-oracle: FAILED: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
