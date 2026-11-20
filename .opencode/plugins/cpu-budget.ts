// @ts-nocheck
import { heavyCommandRefusal } from "../lib/heavy-command.mjs"

export const CpuBudget = async () => ({
  "tool.execute.before": async (input, output) => {
    if (input.tool !== "bash") {
      return
    }
    const refusal = heavyCommandRefusal(output?.args?.command)
    if (refusal) {
      throw new Error(refusal)
    }
  },
})
