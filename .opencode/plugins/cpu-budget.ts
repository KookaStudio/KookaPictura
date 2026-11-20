// @ts-nocheck
import { spawnSync } from "node:child_process"
import { existsSync } from "node:fs"
import { join } from "node:path"

export const CpuBudget = async ({ directory, worktree }) => ({
  "tool.execute.before": async (input, output) => {
    if (input.tool !== "bash") {
      return
    }
    const command = output?.args?.command
    if (typeof command !== "string" || command.trim() === "") {
      return
    }
    const policy = [worktree, directory, process.cwd()]
      .filter(Boolean)
      .map((root) => join(root, "scripts", "heavy_command_policy.py"))
      .find((path) => existsSync(path))
    if (!policy) {
      return
    }
    const result = spawnSync("python3", [policy, "--command", command], {
      encoding: "utf8",
      timeout: 5000,
    })
    if (result.status === 2) {
      throw new Error((result.stderr || "").trim() || "Refused by the CPU budget policy")
    }
  },
})
