from typing import Optional


_current_connector: Optional[str] = None  # "universal" or "reference"


def set_current_connector(name: str) -> None:
    global _current_connector
    _current_connector = name


def is_new_driver() -> bool:
    return _current_connector == "universal"


def is_old_driver() -> bool:
    return _current_connector == "reference"


def NEW_DRIVER_ONLY(bc_id: str) -> bool:
    return is_new_driver()


def OLD_DRIVER_ONLY(bc_id: str) -> bool:
    return is_old_driver()
