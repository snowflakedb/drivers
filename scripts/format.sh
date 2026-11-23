#!/usr/bin/env bash
# Apply formatters for the touched subsystems, then run the repo's pre-commit hooks on the same files.
set -euo pipefail

# shellcheck source=scripts/lib/test-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/test-common.sh"

usage() {
    cat <<'EOF'
Usage: scripts/format.sh [--staged | --all | FILE...] [--no-fix] [--hook ID]

With no file arguments, runs on every file that differs from HEAD:
staged, unstaged and untracked (deleted files are skipped).

  --staged    only staged files
  --all       every file in the repo (pre-commit --all-files)
  --no-fix    skip the fix step; only run the pre-commit hooks
  --hook ID   skip the fix step and run a single hook from
              .pre-commit-config.yaml, e.g. rust-linting

Fix step, per touched subsystem (the matching pre-commit hooks only check):
  python/**/*.py           cd python && hatch run precommit:fix
  jdbc/**/*.java|*.gradle  cd jdbc && ./gradlew spotlessApply
  dotnet/**/*.cs|...       cd dotnet && dotnet format
Rust, C/C++ and Node.js hooks rewrite files themselves.

Then `pre-commit run --files <files>` runs every hook whose files:/types:
filter matches. A hook that rewrote files reports Failed; rerun until clean.
EOF
}

MODE=changed
FIX=1
HOOK=()
FILES=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage; exit 0 ;;
        --staged) MODE=staged ;;
        --all) MODE=all ;;
        --no-fix) FIX=0 ;;
        --hook) HOOK=("${2:?--hook needs a value}"); FIX=0; shift ;;
        -*) die "unknown option $1" ;;
        *) MODE=files; FILES+=("$1") ;;
    esac
    shift
done

require_tool pre-commit "install with: uv tool install pre-commit"
cd "$REPO_ROOT"

add_existing() {
    local f
    while IFS= read -r f; do
        if [[ -n "$f" && -e "$f" ]]; then
            FILES+=("$f")
        fi
    done
}

case "$MODE" in
    staged)
        add_existing < <(git diff --cached --name-only)
        ;;
    changed)
        add_existing < <(git diff --name-only HEAD)
        add_existing < <(git ls-files --others --exclude-standard)
        ;;
esac

if [[ "$MODE" != all && ${#FILES[@]} -eq 0 ]]; then
    echo "No changed files to check."
    exit 0
fi

touches() {
    local pattern="$1" f
    [[ "$MODE" == all ]] && return 0
    for f in "${FILES[@]}"; do
        if [[ "$f" =~ $pattern ]]; then
            return 0
        fi
    done
    return 1
}

if [[ "$FIX" == 1 ]]; then
    if touches '^python/.*\.py$'; then
        (cd python && run hatch run precommit:fix)
    fi
    if touches '^jdbc/(.*\.(java|gradle)|gradle\.properties)$'; then
        (cd jdbc && run ./gradlew spotlessApply)
    fi
    if touches '^dotnet/.*\.(cs|csproj|sln|props|targets|editorconfig)$'; then
        (
            cd dotnet
            if [[ ! src/Snowflake.Data.Proto/DatabaseDriverV1.cs -nt ../protobuf/database_driver_v1.proto ]]; then
                run dotnet msbuild -t:RegenerateProto -v:q -nologo src/Snowflake.Data.Proto/
            fi
            run dotnet format
        )
    fi
fi

if [[ "$MODE" == all ]]; then
    run pre-commit run ${HOOK[@]+"${HOOK[@]}"} --all-files
else
    run pre-commit run ${HOOK[@]+"${HOOK[@]}"} --files "${FILES[@]}"
fi
