def _check_if_universal():
    try:
        # Import universal driver specific code
        from snowflake.connector._internal.api_client.client_api import core_driver  # noqa

        return True
    except ImportError:
        return False


IS_UNIVERSAL_DRIVER = _check_if_universal()


def is_new_driver() -> bool:
    return IS_UNIVERSAL_DRIVER


def is_old_driver() -> bool:
    return not IS_UNIVERSAL_DRIVER


def native_arrow_enabled() -> bool:
    if not IS_UNIVERSAL_DRIVER:
        return False
    try:
        from snowflake.connector._core import sf_core_python
    except ImportError:
        return False
    return bool(sf_core_python.native_arrow_enabled())


def NEW_DRIVER_ONLY(bc_id: str) -> bool:
    return is_new_driver()


def OLD_DRIVER_ONLY(bc_id: str) -> bool:
    return is_old_driver()
