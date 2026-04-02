# Breaking Changes

## `ConfigSourceError` hierarchy change

**Affected version:** SNOW-3264508

`ConfigSourceError` previously inherited from `ConfigManagerError`. It now
inherits directly from `Error`, making it a sibling of `ConfigManagerError`
rather than a subclass.

**Before:**
```
Error
└── ConfigManagerError
    └── ConfigSourceError
        └── MissingConfigOptionError
```

**After:**
```
Error
├── ConfigManagerError
└── ConfigSourceError
    └── MissingConfigOptionError
```

**Impact:** Code that catches `ConfigManagerError` expecting to also catch
`ConfigSourceError` (or `MissingConfigOptionError`) will no longer work.

**Migration:** Add an explicit `except ConfigSourceError` clause wherever
you previously relied on `ConfigManagerError` to catch config source errors:

```python
# Before
try:
    config_manager_op()
except ConfigManagerError as e:
    handle(e)

# After
try:
    config_manager_op()
except (ConfigManagerError, ConfigSourceError) as e:
    handle(e)
```
