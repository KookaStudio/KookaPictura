const HEAVY = /(^|[;&|]\s*)(cargo|cmake|ctest|ninja)\b/m

const EXEMPT = [
  "cpu-budget.py",
  "with-build-lock.sh",
  "verify-fast.sh",
  "verify-full.sh",
  "test-report.sh",
]

const COMPOUND = /[;&|]|\b(?:bash|sh)\s+-c\b/

// ponytail: segment match only; env-prefixed, absolute-path, and non-opencode
// invocations bypass it. Prose guidance and the wrapper remain the baseline.
export function heavyCommandRefusal(command) {
  if (typeof command !== "string" || command.trim() === "") {
    return null
  }
  if (EXEMPT.some((name) => command.includes(name))) {
    return null
  }
  if (!HEAVY.test(command)) {
    return null
  }
  const wrapped = COMPOUND.test(command)
    ? `bash scripts/with-build-lock.sh -- bash -c '${command}'`
    : `bash scripts/with-build-lock.sh -- ${command}`
  return `Refused a bare heavy command. Run it through the CPU-budget wrapper:\n  ${wrapped}`
}
