add_executable(setup_readonly_db setup_readonly_db.cpp)
target_link_libraries(setup_readonly_db PRIVATE common Threads::Threads)
catch_discover_tests(setup_readonly_db TEST_PREFIX "setup_readonly_db: ")
