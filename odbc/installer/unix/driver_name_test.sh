#!/bin/bash
#
# Runs the install, removal, and rpm post-transaction scripts against a stub
# odbcinst and a temp tree, in the order rpm and deb run them.
#

set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SRC/../../.." && pwd)"

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

rewrite_script() {
    local src="$1" dest="$2"
    awk -v from="/usr/lib64/snowflake/odbc" -v to="$ODBC_ROOT" '{gsub(from, to); print}' "$src" > "$dest"
    chmod +x "$dest"
}

setup() {
    WORK="$(mktemp -d)"
    ODBC_ROOT="$WORK/odbc"
    mkdir -p "$ODBC_ROOT/templates" "$ODBC_ROOT/lib" "$WORK/bin" "$WORK/sections"
    cp "$SRC/driver_name.sh" "$ODBC_ROOT/driver_name.sh"
    cp "$REPO/odbc/installer/shared/templates/odbcinst.ini.template" "$ODBC_ROOT/templates/"
    cp "$REPO/odbc/installer/shared/templates/odbc.ini.template" "$ODBC_ROOT/templates/"
    rewrite_script "$SRC/after_install.sh" "$WORK/after_install.sh"
    rewrite_script "$SRC/before_remove.sh" "$WORK/before_remove.sh"
    rewrite_script "$SRC/rpm_posttrans.sh" "$WORK/rpm_posttrans.sh"
    export FAKE_SECTIONS="$WORK/sections"
    export FAKE_DRIVERS_INI="$WORK/odbcinst.ini"
    export FAKE_SYSTEM_DSN="$WORK/odbc.ini"

    # The pre-remove shipped before the driver-name record. It runs during
    # an rpm upgrade from such a package, after the new post-install.
    cat > "$WORK/legacy_before_remove.sh" << 'EOF'
#!/bin/bash
DRIVER_NAME="${SF_DRIVER_NAME:-Snowflake ODBC}"

odbcinst -u -d -n "$DRIVER_NAME" || true

SYSTEM_DSN_PATH=$(odbcinst -j | grep "SYSTEM DATA SOURCES" | sed -n -e 's/SYSTEM DATA SOURCES: //p')
OLD_ODBC_INI=${ODBCINI-}
export ODBCINI="$SYSTEM_DSN_PATH"
odbcinst -u -s -l -n snowflake || true
if [[ -z "$OLD_ODBC_INI" ]]; then
    unset ODBCINI
else
    export ODBCINI="$OLD_ODBC_INI"
fi
EOF

    # Driver sections keep a UsageCount: install adds one, uninstall
    # subtracts one and deletes the section at zero. DSNs are not counted.
    cat > "$WORK/bin/odbcinst" << 'EOF'
#!/bin/bash
set -euo pipefail

write_drivers_ini() {
    local file
    : > "$FAKE_DRIVERS_INI"
    for file in "$FAKE_SECTIONS"/*; do
        [[ -e "$file" ]] || continue
        printf '[%s]\nUsageCount=%s\n' "$(basename "$file")" "$(cat "$file")" >> "$FAKE_DRIVERS_INI"
    done
}

name_arg() {
    local prev="" arg
    for arg in "$@"; do
        if [[ "$prev" == "-n" ]]; then
            printf '%s' "$arg"
            return
        fi
        prev="$arg"
    done
}

case "$1 ${2:-}" in
    "-j ")
        printf 'unixODBC stub\nDRIVERS............: %s\nSYSTEM DATA SOURCES: %s\n' \
            "$FAKE_DRIVERS_INI" "$FAKE_SYSTEM_DSN"
        ;;
    "-i -d")
        IFS= read -r header
        cat > /dev/null
        name="${header#[}"
        name="${name%]}"
        count=0
        if [[ -f "$FAKE_SECTIONS/$name" ]]; then
            count="$(cat "$FAKE_SECTIONS/$name")"
        fi
        printf '%s\n' "$((count + 1))" > "$FAKE_SECTIONS/$name"
        write_drivers_ini
        ;;
    "-u -d")
        name="$(name_arg "$@")"
        if [[ -f "$FAKE_SECTIONS/$name" ]]; then
            count="$(cat "$FAKE_SECTIONS/$name")"
            if (( count <= 1 )); then
                rm -f "$FAKE_SECTIONS/$name"
            else
                printf '%s\n' "$((count - 1))" > "$FAKE_SECTIONS/$name"
            fi
        fi
        write_drivers_ini
        ;;
    "-i -s")
        cat > "$FAKE_SYSTEM_DSN"
        ;;
    "-u -s")
        if grep -F -x -q -- "[$(name_arg "$@")]" "$FAKE_SYSTEM_DSN"; then
            : > "$FAKE_SYSTEM_DSN"
        fi
        ;;
esac
EOF
    chmod +x "$WORK/bin/odbcinst"
    export PATH="$WORK/bin:${PATH:-}"
}

cleanup() {
    if [[ -n "${WORK:-}" && -d "$WORK" ]]; then
        rm -rf "$WORK"
    fi
}
trap cleanup EXIT

reset_state() {
    rm -f "$FAKE_SECTIONS"/* "$ODBC_ROOT/driver-name"
    : > "$FAKE_DRIVERS_INI"
    : > "$FAKE_SYSTEM_DSN"
    unset SF_DRIVER_NAME SF_ACCOUNT
}

legacy_install() {
    printf '[%s]\n' "$1" | odbcinst -i -d -r
    printf '[snowflake]\n' | odbcinst -i -s -l -r
}

run() {
    if ! bash "$WORK/$1" "${@:2}" > "$WORK/run.log" 2>&1; then
        cat "$WORK/run.log" >&2
        fail "$* exited non-zero"
    fi
}

assert_count() {
    local name="$1" expected="$2" actual=0
    if [[ -f "$FAKE_SECTIONS/$name" ]]; then
        actual="$(cat "$FAKE_SECTIONS/$name")"
    fi
    [[ "$actual" == "$expected" ]] || fail "$3: UsageCount of '$name' is $actual, expected $expected"
}

assert_dsn() {
    if [[ "$1" == present ]]; then
        grep -F -x -q '[snowflake]' "$FAKE_SYSTEM_DSN" || fail "$2: snowflake DSN missing"
        if [[ -n "${3:-}" ]]; then
            grep -F -x -q "Driver=$3" "$FAKE_SYSTEM_DSN" || fail "$2: snowflake DSN does not use driver '$3'"
        fi
    else
        ! grep -F -x -q '[snowflake]' "$FAKE_SYSTEM_DSN" || fail "$2: snowflake DSN still present"
    fi
}

assert_record() {
    local actual=""
    if [[ -f "$ODBC_ROOT/driver-name" ]]; then
        actual="$(tr -d '\n' < "$ODBC_ROOT/driver-name")"
    fi
    [[ "$actual" == "$1" ]] || fail "$2: record is '$actual', expected '$1'"
}

setup

reset_state
printf '[Snowflake ODBC]\n' | odbcinst -i -d -r
export SF_DRIVER_NAME='Custom Name'
run after_install.sh 1
run rpm_posttrans.sh
assert_count 'Custom Name' 1 'custom install'
assert_count 'Snowflake ODBC' 1 'custom install leaves another registration'
assert_record 'Custom Name' 'custom install'
assert_dsn present 'custom install' 'Custom Name'
export SF_DRIVER_NAME='Other Name'
run before_remove.sh 0
assert_count 'Custom Name' 0 'rpm erase'
assert_count 'Other Name' 0 'rpm erase ignores SF_DRIVER_NAME'
assert_count 'Snowflake ODBC' 1 'rpm erase leaves another registration'
assert_record '' 'rpm erase'
assert_dsn absent 'rpm erase'

for removal in remove ''; do
    reset_state
    export SF_DRIVER_NAME='Deb Name'
    run after_install.sh configure
    unset SF_DRIVER_NAME
    if [[ -n "$removal" ]]; then
        run before_remove.sh "$removal"
    else
        run before_remove.sh
    fi
    assert_count 'Deb Name' 0 "removal '$removal'"
    assert_record '' "removal '$removal'"
    assert_dsn absent "removal '$removal'"
done

reset_state
run after_install.sh 1
run rpm_posttrans.sh
assert_count 'Snowflake ODBC' 1 'default install'
assert_record 'Snowflake ODBC' 'default install'

reset_state
export SF_DRIVER_NAME=' A&B|C\D '
run after_install.sh 1
run rpm_posttrans.sh
unset SF_DRIVER_NAME
run after_install.sh 2
run before_remove.sh 1
run rpm_posttrans.sh
assert_count 'A&B|C\D' 1 'special-character name upgrade keeps one registration'
assert_record 'A&B|C\D' 'special-character name'
assert_dsn present 'special-character name' 'A&B|C\D'
run before_remove.sh 0
assert_count 'A&B|C\D' 0 'special-character name erase'

reset_state
legacy_install 'Legacy Name'
export SF_DRIVER_NAME='Legacy Name'
run before_remove.sh 0
assert_count 'Legacy Name' 0 'removal with no record falls back to SF_DRIVER_NAME'

reset_state
export SF_DRIVER_NAME='Kept Name'
run after_install.sh 1
unset SF_DRIVER_NAME
for upgrade in 1 2 3; do
    run after_install.sh 2
    run before_remove.sh 1
    run rpm_posttrans.sh
    assert_count 'Kept Name' 1 "rpm upgrade $upgrade keeps one registration"
    assert_dsn present "rpm upgrade $upgrade" 'Kept Name'
done
run before_remove.sh 0
assert_count 'Kept Name' 0 'rpm erase after upgrades'

reset_state
export SF_DRIVER_NAME='Kept Name'
run after_install.sh configure
unset SF_DRIVER_NAME
for upgrade in 1 2 3; do
    run before_remove.sh upgrade 4.3.0
    run after_install.sh configure 4.2.0
    assert_count 'Kept Name' 1 "deb upgrade $upgrade keeps one registration"
    assert_dsn present "deb upgrade $upgrade" 'Kept Name'
done
run before_remove.sh remove
assert_count 'Kept Name' 0 'deb remove after upgrades'

reset_state
export SF_DRIVER_NAME='Old Rpm'
run after_install.sh 1
export SF_DRIVER_NAME='New Rpm'
run after_install.sh 2
run before_remove.sh 1
run rpm_posttrans.sh
assert_count 'Old Rpm' 0 'rpm rename upgrade'
assert_count 'New Rpm' 1 'rpm rename upgrade'
assert_record 'New Rpm' 'rpm rename upgrade'
assert_dsn present 'rpm rename upgrade' 'New Rpm'

reset_state
export SF_DRIVER_NAME='Old Deb'
run after_install.sh configure
export SF_DRIVER_NAME='New Deb'
run before_remove.sh upgrade 4.3.0
assert_count 'Old Deb' 1 'deb prerm upgrade'
run after_install.sh configure 4.2.0
assert_count 'Old Deb' 0 'deb rename upgrade'
assert_count 'New Deb' 1 'deb rename upgrade'
assert_record 'New Deb' 'deb rename upgrade'
assert_dsn present 'deb rename upgrade' 'New Deb'

reset_state
legacy_install 'Snowflake ODBC'
run after_install.sh 2
run legacy_before_remove.sh 1
assert_count 'Snowflake ODBC' 1 'legacy rpm pre-remove after post-install'
assert_dsn absent 'legacy rpm pre-remove after post-install'
run rpm_posttrans.sh
assert_count 'Snowflake ODBC' 1 'legacy rpm upgrade'
assert_dsn present 'legacy rpm upgrade' 'Snowflake ODBC'
run before_remove.sh 0
assert_count 'Snowflake ODBC' 0 'rpm erase after legacy upgrade'
assert_dsn absent 'rpm erase after legacy upgrade'

reset_state
legacy_install 'Legacy Custom'
run after_install.sh 2
run legacy_before_remove.sh 1
assert_count 'Snowflake ODBC' 0 'legacy rpm pre-remove deletes the new default'
run rpm_posttrans.sh
assert_count 'Snowflake ODBC' 1 'legacy custom rpm upgrade restores the default'
assert_count 'Legacy Custom' 1 'legacy custom name stays registered'
assert_dsn present 'legacy custom rpm upgrade' 'Snowflake ODBC'

reset_state
legacy_install 'Legacy Custom'
export SF_DRIVER_NAME='Legacy Custom'
run after_install.sh 2
run legacy_before_remove.sh 1
run rpm_posttrans.sh
assert_count 'Legacy Custom' 1 'legacy custom rpm upgrade with SF_DRIVER_NAME'
assert_record 'Legacy Custom' 'legacy custom rpm upgrade with SF_DRIVER_NAME'
assert_dsn present 'legacy custom rpm upgrade with SF_DRIVER_NAME' 'Legacy Custom'

reset_state
legacy_install 'Snowflake ODBC'
run legacy_before_remove.sh upgrade 4.3.0
run after_install.sh configure 4.1.0
assert_count 'Snowflake ODBC' 1 'legacy deb upgrade'
assert_dsn present 'legacy deb upgrade' 'Snowflake ODBC'

reset_state
legacy_install 'Legacy Custom'
run legacy_before_remove.sh upgrade 4.3.0
run after_install.sh configure 4.1.0
assert_count 'Snowflake ODBC' 1 'legacy custom deb upgrade registers the default'
assert_count 'Legacy Custom' 1 'legacy custom name stays registered after deb upgrade'
assert_record 'Snowflake ODBC' 'legacy custom deb upgrade'
assert_dsn present 'legacy custom deb upgrade' 'Snowflake ODBC'

printf '%s\n' 'ok'
