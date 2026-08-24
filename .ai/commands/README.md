# Commands Rules

Command rules are action-oriented instructions that tell AI tools how to perform specific tasks.

**📚 [Complete Documentation](https://snowflakecomputing.atlassian.net/wiki/spaces/DPD/pages/4658659432/sf+CLI+sf+ai+rules)**

## Commands Schema

```markdown
---
description: Brief description (REQUIRED)
argument-hint: "[ARGS]"                                    # Optional, Claude only
model: "us.anthropic.claude-opus-4-5-20251101-v1:0"        # Optional, Claude only
allowed-tools: Read, Write(**/*.md), Bash(git status:*)    # Optional, Claude only (comma-separated)
globs: path/*.ext, path2/*.ext                             # Optional, Cursor only (comma-separated)
inline-content: false                                      # Optional, default: false. If true, copies full content instead of pointer
---

# Your Rule Title

Full instructions here...
```
