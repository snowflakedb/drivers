"""
Integration tests for Connection.converter_class and Connection.converter properties.

The converter_class property returns the class used to convert Snowflake data types
to Python objects. In the reference driver it defaults to SnowflakeConverter; in the
universal driver conversion is handled by the C++ Arrow layer and Rust core, but the
properties are exposed as deprecated no-ops for backward compatibility.
"""

import warnings

import pytest


# Suppress the module-level DeprecationWarning emitted on first import of these
# backward-compat names so collection does not pollute the warnings report.
with warnings.catch_warnings():
    warnings.simplefilter("ignore", DeprecationWarning)
    from snowflake.connector.converter import SnowflakeConverter
    from snowflake.connector.converter_null import SnowflakeNoConverterToPython


def _get_converter_class(connection):
    """Read converter_class while suppressing the DeprecationWarning."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        return connection.converter_class


def _get_converter(connection):
    """Read converter while suppressing the DeprecationWarning."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        return connection.converter


class TestConverterClassProperty:
    """Integration tests for Connection.converter_class."""

    def test_converter_class_returns_a_type(self, connection):
        """converter_class should return a class (type), not an instance."""
        assert isinstance(_get_converter_class(connection), type)

    def test_converter_class_is_snowflake_converter_or_subclass(self, connection):
        """The default converter_class should be SnowflakeConverter or a subclass of it."""
        assert issubclass(_get_converter_class(connection), SnowflakeConverter)

    def test_converter_class_accessible_on_open_connection(self, connection):
        """converter_class should be accessible on an open connection without error."""
        cls = _get_converter_class(connection)
        assert cls is not None

    def test_converter_class_on_closed_connection(self, connection_factory):
        """converter_class should remain accessible after the connection is closed."""
        conn = connection_factory()
        cls_before = _get_converter_class(conn)
        conn.close()
        cls_after = _get_converter_class(conn)
        assert cls_before is cls_after

    def test_converter_class_custom_subclass(self, connection_factory):
        """Passing a custom converter_class subclass should be accepted."""

        class MyConverter(SnowflakeConverter):
            pass

        with connection_factory(converter_class=MyConverter) as conn:
            assert _get_converter_class(conn) is MyConverter

    def test_no_converter_to_python_is_subclass_of_snowflake_converter(self):
        """SnowflakeNoConverterToPython should be a subclass of SnowflakeConverter."""
        assert issubclass(SnowflakeNoConverterToPython, SnowflakeConverter)

    def test_no_converter_to_python_as_converter_class(self, connection_factory):
        """Passing SnowflakeNoConverterToPython as converter_class should be accepted."""
        with connection_factory(converter_class=SnowflakeNoConverterToPython) as conn:
            assert _get_converter_class(conn) is SnowflakeNoConverterToPython

    @pytest.mark.skip_reference(reason="DeprecationWarning is universal-driver only")
    def test_converter_class_emits_deprecation_warning(self, connection):
        """Accessing converter_class should emit a DeprecationWarning."""
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            _ = connection.converter_class
        deprecations = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert len(deprecations) == 1
        assert "converter_class" in str(deprecations[0].message)


class TestConverterProperty:
    """Integration tests for Connection.converter (instance)."""

    def test_converter_is_instance_of_converter_class(self, connection):
        """converter should be an instance of the connection's converter_class."""
        assert isinstance(_get_converter(connection), _get_converter_class(connection))

    def test_converter_default_is_snowflake_converter_instance(self, connection):
        """With default settings, converter should be a SnowflakeConverter instance."""
        assert isinstance(_get_converter(connection), SnowflakeConverter)

    def test_converter_with_custom_class(self, connection_factory):
        """When converter_class is overridden, converter should be an instance of that class."""
        with connection_factory(converter_class=SnowflakeNoConverterToPython) as conn:
            assert isinstance(_get_converter(conn), SnowflakeNoConverterToPython)

    @pytest.mark.skip_reference(reason="DeprecationWarning is universal-driver only")
    def test_converter_emits_deprecation_warning(self, connection):
        """Accessing converter should emit a DeprecationWarning."""
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            _ = connection.converter
        deprecations = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert len(deprecations) == 1
        assert "converter" in str(deprecations[0].message)
