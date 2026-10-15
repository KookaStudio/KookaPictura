---
description: Read-only GitHub investigator for issues, PRs, CI status
mode: subagent
model: anthropic/claude-haiku-4-5
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