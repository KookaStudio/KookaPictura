---
description: Read-only GitHub investigator for issues, PRs, CI status
mode: subagent
model: opencode/deepseek-v4.1-flash-free
tools:
  write: false
  edit: false
permission:
  bash:
    "*": deny
    "gh issue view*": allow
    "gh pr view*": allow
    "gh pr list*": allow
    "gh pr checks*": allow
    "gh status*": allow
---
Same instructions as above.

## Model fallback

If a GitHub read fails or stalls because this agent's configured model
(`opencode/deepseek-v4.1-flash-free`) errors or produces no output for about
two minutes (provider failure, rate limit, tool-call failure, or a hung
stream), retry the same read once. If it still fails or stalls, stop and report
the failure to the caller instead of looping. After two failed or hung
`gh-reader` attempts the caller stops delegating and reads GitHub with its own
current model.
