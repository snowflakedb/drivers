# AI Rules Management

The `.ai/` directory is the **single source of truth** for all AI rules, eliminating duplication across different AI tools.

## Documentation

For complete documentation on the `.ai/` commands refer to the `sf ai rules` command and workflow, see:

**📚 [sf CLI - sf ai rules (Official Documentation)](https://snowflakecomputing.atlassian.net/wiki/spaces/DPD/pages/4658659432/sf+CLI+sf+ai+rules)**

## Quick Reference

### Directory Structure

```
.ai/
├── commands/      # Action-oriented rules
│   └── README.md  # Schema documentation
├── context/       # Used for providing useful context for AI agents
└── review/        # PR review rules
```

### Key Principles

- **Single source of truth**: Always edit `.ai/` files, never generated files in `.claude/` or `.cursor/`
- **Generated pointers**: Build process creates lightweight pointer files that reference `.ai/` content
- **Safety first**: Linter prevents dangerous configurations (e.g., `alwaysApply: true`)
