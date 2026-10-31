---
description: Read-only GitHub investigator for issues, PRs, CI status
mode: subagent
model: deepinfra/deepseek-ai/DeepSeek-V4.1-Flash
tools:
  write: false
  edit: false
permission:
  bash:
    "gh issue view*": allow
    "gh pr view*": allow
    "gh pr list*": allow
    "gh pr checks*": allow
    "gh status*": allow
    "*": deny
---
Same instructions as above.