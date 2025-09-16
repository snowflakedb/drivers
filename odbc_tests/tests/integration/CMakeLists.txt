# Integration tests - create individual tests for each .cpp file

# Find all .cpp files recursively
file(GLOB_RECURSE all_sources "*.cpp")

# Create a test for each .cpp file
foreach(source_file ${all_sources})
  # Get the relative path from current directory
  file(RELATIVE_PATH rel_path ${CMAKE_CURRENT_SOURCE_DIR} ${source_file})
  
  # Get filename without extension for test name
  get_filename_component(file_name ${rel_path} NAME_WE)
  
  # Get the directory name to include in test name
  get_filename_component(dir_name ${rel_path} DIRECTORY)
  
  # Create test name: integration_directory_filename
  if(dir_name)
    string(REPLACE "/" "_" dir_clean ${dir_name})
    set(test_name "integration_${dir_clean}_${file_name}")
  else()
    set(test_name "integration_${file_name}")
  endif()
  
  # Create individual test for this file
  add_odbc_test(${test_name} ${source_file})
  message(STATUS "Added Integration test: ${test_name} from ${rel_path}")
endforeach()
