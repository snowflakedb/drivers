# Driver name recorded at install and read back at removal, and the unixODBC
# registration steps shared by the package scripts.
# The caller sets ODBC_DIR before sourcing this file.
# rpm runs an installed package's pre-remove after the next package's files
# are in place, so that pre-remove sources the next package's copy of this
# file and depends on the functions it calls staying here.

driver_name_record_file() {
    printf '%s/driver-name' "$ODBC_DIR"
}

read_recorded_driver_name() {
    local file name
    file="$(driver_name_record_file)"
    if [[ ! -f "$file" ]]; then
        return 1
    fi
    IFS= read -r name < "$file" || true
    if [[ -z "$name" ]]; then
        return 1
    fi
    printf '%s' "$name"
}

write_recorded_driver_name() {
    printf '%s\n' "$1" > "$(driver_name_record_file)"
}

delete_recorded_driver_name() {
    rm -f "$(driver_name_record_file)"
}

# odbcinst strips surrounding whitespace from a section name, so the recorded
# name is stripped the same way.
install_driver_name() {
    local name
    name="$(printf '%s' "${SF_DRIVER_NAME:-}" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
    if [[ -n "$name" ]]; then
        printf '%s' "$name"
        return
    fi
    if read_recorded_driver_name; then
        return
    fi
    printf '%s' "Snowflake ODBC"
}

# The record names the driver registered at install, which SF_DRIVER_NAME in
# the removal environment does not have to match.
removal_driver_name() {
    if read_recorded_driver_name; then
        return
    fi
    if [[ -n "${SF_DRIVER_NAME:-}" ]]; then
        printf '%s' "$SF_DRIVER_NAME"
        return
    fi
    printf '%s' "Snowflake ODBC"
}

# rpm %preun passes 0 on erase and a positive count on upgrade.
# deb prerm passes "remove" on removal; "upgrade" and its other arguments are
# not a full removal. No argument is a full removal.
full_removal() {
    case "${1:-remove}" in
        0|remove) return 0 ;;
        *) return 1 ;;
    esac
}

dsn_account() {
    if [[ -n "${SF_ACCOUNT:-}" ]]; then
        printf '%s' "$SF_ACCOUNT"
        return
    fi
    echo "[WARN] SF_ACCOUNT is not set, please manually update the odbc.ini file after installation" >&2
    printf '%s' SF_ACCOUNT
}

sed_replacement() {
    printf '%s' "$1" | sed -e 's/[\\|&]/\\&/g'
}

render_template() {
    sed \
        -e "s|__DRIVER_NAME__|$(sed_replacement "$1")|g" \
        -e "s|__DRIVER_PATH__|$(sed_replacement "$ODBC_DIR/lib/libsfodbc.so")|g" \
        -e "s|__SF_ACCOUNT__|$(sed_replacement "$2")|g" \
        "$ODBC_DIR/templates/$3"
}

odbcinst_file() {
    odbcinst -j | sed -n -e "s/^$1\.*: //p"
}

ini_has_section() {
    [[ -n "$1" && -f "$1" ]] && grep -i -F -x -q -- "[$2]" "$1"
}

driver_registered() {
    ini_has_section "$(odbcinst_file DRIVERS)" "$1"
}

dsn_registered() {
    ini_has_section "$(odbcinst_file 'SYSTEM DATA SOURCES')" snowflake
}

# unixODBC's odbcinst keeps a UsageCount on each driver section: an install
# adds one and an uninstall subtracts one, deleting the section at zero.
register_driver() {
    render_template "$1" "" odbcinst.ini.template | odbcinst -i -d -r
}

# Rewrites an existing section from the template without changing its count.
refresh_driver() {
    if ! driver_registered "$1"; then
        register_driver "$1"
        return
    fi
    if register_driver "$1"; then
        odbcinst -u -d -n "$1" || true
    fi
}

register_dsn() {
    render_template "$1" "$2" odbc.ini.template | odbcinst -i -s -l -r
}
