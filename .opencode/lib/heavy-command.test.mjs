import assert from "node:assert/strict"
import test from "node:test"

import { heavyCommandRefusal } from "./heavy-command.mjs"

const refused = (command) => {
  const message = heavyCommandRefusal(command)
  assert.ok(message, `expected a refusal for ${command}`)
  return message
}

test("bare heavy segments are refused", () => {
  for (const command of [
    "cargo build",
    "cargo nextest run --workspace",
    "cmake -S . -B build",
    "ctest --test-dir build",
    "ninja -C build",
  ]) {
    assert.match(refused(command), /bash scripts\/with-build-lock\.sh -- /)
  }
})

test("a heavy segment after a shell operator is refused", () => {
  assert.match(refused("git status && cargo test"), /with-build-lock\.sh/)
  assert.match(refused("true; ctest"), /with-build-lock\.sh/)
  assert.match(refused("ninja -C build | tee log"), /with-build-lock\.sh/)
})

test("a compound command names the bash -c form", () => {
  const message = refused("git status && cargo test")
  assert.match(message, /bash scripts\/with-build-lock\.sh -- bash -c '/)
})

test("a simple command names the direct form", () => {
  const message = refused("cargo build")
  assert.match(message, /bash scripts\/with-build-lock\.sh -- cargo build/)
})

test("wrapped, gate, and non-heavy commands pass", () => {
  for (const command of [
    "bash scripts/with-build-lock.sh -- cargo build",
    "python3 scripts/cpu-budget.py exec -- cargo test",
    "bash scripts/verify-fast.sh",
    "bash scripts/verify-full.sh",
    "bash scripts/test-report.sh never",
    "git status",
    "echo cargo",
  ]) {
    assert.equal(heavyCommandRefusal(command), null, command)
  }
})

test("cheap cargo fmt segments pass", () => {
  for (const command of ["cargo fmt --all", "cargo fmt && git status"]) {
    assert.equal(heavyCommandRefusal(command), null, command)
  }
})

test("a cheap segment does not exempt a heavy sibling", () => {
  assert.match(refused("cargo fmt --all && cargo build"), /with-build-lock\.sh/)
})

test("empty and non-string input pass", () => {
  assert.equal(heavyCommandRefusal(""), null)
  assert.equal(heavyCommandRefusal(undefined), null)
})
