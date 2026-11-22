// @ts-nocheck
import { spawnSync } from "node:child_process"
import { accessSync, constants, existsSync, readFileSync, realpathSync, statSync } from "node:fs"
import { dirname, isAbsolute, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

function projectOptIn() {
  const { HOME, KOOKA_ENV_FILE, XDG_CONFIG_HOME } = process.env
  const configDir = XDG_CONFIG_HOME || (HOME && join(HOME, ".config"))
  const settings = KOOKA_ENV_FILE || (configDir && join(configDir, "kooka-pictura", "env"))
  if (!settings) {
    return false
  }
  let text
  try {
    if (!statSync(settings).isFile()) {
      return false
    }
    text = readFileSync(settings, "utf8")
  } catch {
    return false
  }
  const matches = [...text.matchAll(/^[ \t]*(?:export[ \t]+)?PICTURA_OPENCODE_MANAGED=([^\n]*)$/gm)]
  return /^(?:1|'1'|"1")[ \t]*(?:#.*)?$/.test(matches.at(-1)?.[1].trim() ?? "")
}

function gitCommonDirectory(cwd) {
  const env = { ...process.env }
  delete env.GIT_DIR
  delete env.GIT_COMMON_DIR
  delete env.GIT_WORK_TREE
  const result = spawnSync("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"], {
    cwd,
    encoding: "utf8",
    env,
    timeout: 5000,
  })
  if (result.status !== 0) {
    return undefined
  }
  try {
    return realpathSync(result.stdout.trim())
  } catch {
    return undefined
  }
}

function projectShellPath(directory) {
  if (process.platform !== "linux" || process.env.PICTURA_AGENT_POLICY !== undefined || !projectOptIn()) {
    return undefined
  }
  const checkout = resolve(dirname(realpathSync(fileURLToPath(import.meta.url))), "../..")
  const common = gitCommonDirectory(checkout)
  if (!common || !directory || common !== gitCommonDirectory(directory)) {
    return undefined
  }
  return join(dirname(common), "scripts", "opencode-agent-shell")
}

function requireExecutable(shell) {
  try {
    if (statSync(shell).isFile()) {
      accessSync(shell, constants.X_OK)
      return
    }
  } catch {
    // Both a missing wrapper and one without execute permission block the tool.
  }
  throw new Error("Managed OpenCode shell is missing or not executable: " + shell)
}

export const CpuBudget = async ({ directory, worktree }) => {
  const projectShell = projectShellPath(directory)
  let config
  const verifyProjectShell = () => {
    requireExecutable(projectShell)
    if (config?.shell !== projectShell) {
      throw new Error("Managed OpenCode shell was not selected: " + projectShell)
    }
  }

  return {
    async config(cfg) {
      if (projectShell) {
        config = cfg
        requireExecutable(projectShell)
        cfg.shell = projectShell
      }
    },
    // A config-hook error is logged by OpenCode and can leave its default shell selected.
    "shell.env": async () => {
      if (projectShell) {
        verifyProjectShell()
      }
    },
    "tool.execute.before": async (input, output) => {
      if (input.tool !== "bash") {
        return
      }
      if (projectShell) {
        verifyProjectShell()
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
  }
}
