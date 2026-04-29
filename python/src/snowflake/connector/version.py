# PEP 440 compliant version string (used by hatch for packaging)
__version__ = "5.0.0b1"


def _release_components(version: str) -> tuple[int, ...]:
    components = []
    for segment in version.split("."):
        numeric = ""
        for char in segment:
            if char.isdigit():
                numeric += char
            else:
                break
        if not numeric:
            break
        components.append(int(numeric))
        if len(numeric) != len(segment):
            break
    return tuple(components)


# Compatibility with old driver pattern
VERSION = (*_release_components(__version__), None)
