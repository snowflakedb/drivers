#include "ObjectConverter.hpp"

#include <memory>

#include "CArrowIterator.hpp"
#include "ConverterUtil.hpp"
#include "SnowflakeType.hpp"

namespace sf {
Logger* ObjectConverter::logger = new Logger("snowflake.connector.BinaryConverter");

ObjectConverter::ObjectConverter(ArrowSchemaView* schemaView, ArrowArrayView* array,
                                 PyObject* context, bool useNumpy) {
  m_array = array;
  m_converters.clear();
  m_property_names.clear();
  m_propertyCount = schemaView->schema->n_children;

  for (int i = 0; i < schemaView->schema->n_children; i++) {
    ArrowSchema* property_schema = schemaView->schema->children[i];

    m_property_names.push_back(property_schema->name);

    ArrowArrayView* child_array = array->children[i];

    m_converters.push_back(
        getConverterFromSchema(property_schema, child_array, context, useNumpy, logger));
  }
}

PyObject* ObjectConverter::toPyObject(int64_t rowIndex) const {
  if (ArrowArrayViewIsNull(m_array, rowIndex)) {
    Py_RETURN_NONE;
  }

  PyObject* dict = PyDict_New();
  if (dict == nullptr) {
    return nullptr;
  }
  for (int i = 0; i < m_propertyCount; i++) {
    PyObject* val = m_converters[i]->toPyObject(rowIndex);
    if (val == nullptr || PyDict_SetItemString(dict, m_property_names[i], val) != 0) {
      Py_XDECREF(val);
      Py_DECREF(dict);
      return nullptr;
    }
    Py_DECREF(val);
  }
  return dict;
}

}  // namespace sf
