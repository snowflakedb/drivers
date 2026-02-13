def _check_if_universal():
    try:
        # Import universal driver specific code
        from snowflake.connector._internal.api_client.client_api import database_driver_client  # noqa

        return True
    except ImportError:
        return False


IS_UNIVERSAL_DRIVER = _check_if_universal()


def is_new_driver() -> bool:
    return IS_UNIVERSAL_DRIVER


def is_old_driver() -> bool:
    return not IS_UNIVERSAL_DRIVER


def NEW_DRIVER_ONLY(bc_id: str) -> bool:
    return is_new_driver()


def OLD_DRIVER_ONLY(bc_id: str) -> bool:
    return is_old_driver()
