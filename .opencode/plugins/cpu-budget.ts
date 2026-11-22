// @ts-nocheck
import { spawnSync } from "node:child_process"
import { existsSync } from "node:fs"
import { isAbsolute, join } from "node:path"

export const CpuBudget = async ({ directory, worktree }) => ({
  "tool.execute.before": async (input, output) => {
    if (input.tool !== "bash") {
      return
    }
    const command = output?.args?.command
    if (typeof command !== "string" || command.trim() === "") {
      return
    }
    const canonical = process.env.PICTURA_AGENT_POLICY
    const policy = canonical ?? [worktree, directory, process.cwd()]
      .filter(Boolean)
      .map((root) => join(root, "scripts", "heavy_command_policy.py"))
      .find((path) => existsSync(path))
    if (canonical !== undefined && (!isAbsolute(canonical) || !existsSync(canonical))) {
      throw new Error(`Managed CPU budget policy is missing or not an absolute path: ${canonical}`)
    }
    if (!policy) {
      return
    }
    const args = canonical === undefined ? [] : ["--managed"]
    const result = spawnSync("python3", [policy, ...args, "--command", command], {
      encoding: "utf8",
      timeout: 5000,
    })
    if (result.error) {
      throw new Error(`Could not run CPU budget policy at ${policy}: ${result.error.message}`)
    }
    if (result.status === 2) {
      throw new Error((result.stderr || "").trim() || "Refused by the CPU budget policy")
    }
    if (result.status !== 0) {
      const reason = result.signal ? `signal ${result.signal}` : `status ${result.status}`
      throw new Error(`CPU budget policy failed at ${policy}: ${reason}`)
    }
  },
})
