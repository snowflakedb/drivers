@python
Feature: Client-side binding (pyformat/format paramstyles)

  # Python-specific client-side parameter interpolation tests.
  # These test pyformat (%s, %(name)s) and format (%s) paramstyles
  # where parameters are interpolated into the SQL string client-side.

  # =========================================================================== #
  #                   Pyformat positional binding (%s)                          #
  # =========================================================================== #

  @python_e2e
  Scenario: should bind basic types with positional pyformat
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s, %s, %s, %s, %s" is executed with positional parameters [42, 3.14, "hello", True, None]
    Then Result should contain values matching the bound parameters

  @python_e2e
  Scenario: should bind string with single quote
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter "it's a test"
    Then Result should contain the exact string "it's a test"

  @python_e2e
  Scenario: should bind string with double quote
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter containing double quotes
    Then Result should contain the exact string with double quotes

  @python_e2e
  Scenario: should bind string with backslash
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter "path\to\file"
    Then Result should contain the exact string with backslashes

  @python_e2e
  Scenario: should bind string with newline
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter containing newline
    Then Result should contain the exact string with newline

  @python_e2e
  Scenario: should bind string with tab
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter containing tab
    Then Result should contain the exact string with tab

  @python_e2e
  Scenario: should bind string with carriage return
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter containing carriage return
    Then Result should contain the exact string with carriage return

  # =========================================================================== #
  #                   Pyformat named binding (%(name)s)                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should bind basic types with named pyformat
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %(a)s, %(b)s, %(c)s" is executed with named parameters (a=100, b="test", c=True)
    Then Result should contain values matching the bound parameters

  @python_e2e
  Scenario: should bind same parameter multiple times
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %(val)s, %(val)s, %(val)s" is executed with named parameter (val=42)
    Then Result should contain [42, 42, 42]

  @python_e2e
  Scenario: should bind with mixed order named params
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %(z)s, %(a)s, %(m)s" is executed with named parameters (a=1, m=2, z=3)
    Then Result should contain [3, 1, 2]

  # =========================================================================== #
  #                        Escape handling                                     #
  # =========================================================================== #

  @python_e2e
  Scenario: should escape special characters
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (name VARCHAR) exists
    When Strings with special characters (newlines, backslashes, quotes, tabs) are inserted using %s binding
    Then All strings should be retrievable with exact content preserved

  @python_e2e
  Scenario: should prevent sql injection with positional binding
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And Rows are inserted
    When Query with WHERE id = %s is executed with SQL injection string "1 or 1=1"
    Then Error should be raised for numeric type conversion (injection is prevented)

  @python_e2e
  Scenario: should prevent sql injection with named binding
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And Rows are inserted
    When Query with WHERE id = %(id)s is executed with SQL injection string "1 or 1=1"
    Then Error should be raised for numeric type conversion (injection is prevented)

  @python_e2e
  Scenario: should handle complex escape sequence
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with a complex string containing quotes, backslashes, and newlines
    Then Result should contain the exact complex string

  # =========================================================================== #
  #                        Quote handling                                      #
  # =========================================================================== #

  @python_e2e
  Scenario: should quote null as null
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter None
    Then Result should contain NULL

  @python_e2e
  Scenario: should quote boolean true
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter True
    Then Result should contain True

  @python_e2e
  Scenario: should quote boolean false
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter False
    Then Result should contain False

  @python_e2e
  Scenario: should quote integer
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter 12345
    Then Result should contain 12345

  @python_e2e
  Scenario: should quote negative integer
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter -12345
    Then Result should contain -12345

  @python_e2e
  Scenario: should quote float
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter 3.14159
    Then Result should contain approximately 3.14159

  @python_e2e
  Scenario: should quote empty string
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with parameter ""
    Then Result should contain empty string

  @python_e2e
  Scenario: should quote binary data
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s::BINARY" is executed with binary parameter
    Then Result should contain the same binary data

  # =========================================================================== #
  #                        List binding (IN clause)                            #
  # =========================================================================== #

  @python_e2e
  Scenario: should bind list for in clause
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And Rows [1, "Alice"], [2, "Bob"], [3, "Charlie"] are inserted
    When Query with WHERE id IN (%s) is executed with list parameter [1, 3]
    Then Result should contain rows for "Alice" and "Charlie"

  # =========================================================================== #
  #                   Table operations with pyformat                           #
  # =========================================================================== #

  @python_e2e
  Scenario: should insert with positional pyformat
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When Row is inserted using "INSERT INTO table VALUES (%s, %s)" with parameters (1, "Alice")
    Then Table should contain [1, "Alice"]

  @python_e2e
  Scenario: should insert with named pyformat
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When Row is inserted using "INSERT INTO table VALUES (%(id)s, %(name)s)" with named parameters (id=1, name="Alice")
    Then Table should contain [1, "Alice"]

  @python_e2e
  Scenario: should update with pyformat
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And Row [1, "Alice"] is inserted
    When Query "UPDATE table SET name = %s WHERE id = %s" is executed with parameters ("Alice Updated", 1)
    Then Table should contain [1, "Alice Updated"]

  @python_e2e
  Scenario: should delete with pyformat
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    And Rows [1, "Alice"] and [2, "Bob"] are inserted
    When Query "DELETE FROM table WHERE id = %s" is executed with parameter (1)
    Then Table should contain only [2, "Bob"]

  @python_e2e
  Scenario: should select where with pyformat
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR, age NUMBER) exists
    And Rows [1, "Alice", 30], [2, "Bob", 25], [3, "Charlie", 35] are inserted
    When Query "SELECT name FROM table WHERE age > %s ORDER BY name" is executed with parameter (28)
    Then Result should contain rows for "Alice" and "Charlie"

  # =========================================================================== #
  #                   Executemany with pyformat                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should executemany with dict params
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When executemany is called with dict parameters [(id=1, name="Alice"), (id=2, name="Bob"), (id=3, name="Charlie")]
    Then Table should contain 3 rows with correct values

  @python_e2e
  Scenario: should executemany with tuple params
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When executemany is called with tuple parameters [(1, "Alice"), (2, "Bob"), (3, "Charlie")]
    Then Table should contain 3 rows with correct values

  # =========================================================================== #
  #                        Format paramstyle                                   #
  # =========================================================================== #

  @python_e2e
  Scenario: should bind with format style
    Given Snowflake client is logged in with format paramstyle
    When Query "SELECT %s, %s, %s" is executed with parameters (1, "test", True)
    Then Result should contain [1, "test", True]

  @python_e2e
  Scenario: should escape special chars with format
    Given Snowflake client is logged in with format paramstyle
    When Query "SELECT %s" is executed with parameter "it's a 'test'"
    Then Result should contain the exact string "it's a 'test'"

  # =========================================================================== #
  #                        Error handling                                      #
  # =========================================================================== #

  @python_e2e
  Scenario: should raise error for too many positional parameters
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s, %s" is executed with 3 parameters
    Then TypeError should be raised indicating not all arguments converted

  @python_e2e
  Scenario: should raise error for not enough positional parameters
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s, %s, %s" is executed with 2 parameters
    Then TypeError should be raised indicating not enough arguments

  @python_e2e
  Scenario: should raise error for missing named parameter
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %(name)s, %(age)s" is executed with only (name="Alice")
    Then KeyError should be raised for missing parameter

  @python_e2e
  Scenario: should ignore extra named parameters
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %(name)s" is executed with (name="Alice", extra="ignored")
    Then Result should contain "Alice" and no error is raised

  @python_e2e
  Scenario: should raise error for invalid paramstyle
    Given Connection is created with paramstyle "invalid"
    Then ProgrammingError should be raised indicating invalid paramstyle

  @python_e2e
  Scenario: should reject dict params with format paramstyle
    Given Snowflake client is logged in with format paramstyle
    When Query "SELECT %(name)s" is executed with dict parameters
    Then ProgrammingError should be raised indicating dict parameters not supported

  # =========================================================================== #
  #                        Decimal binding                                     #
  # =========================================================================== #

  @python_e2e
  Scenario: should bind decimal value
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s" is executed with Decimal("123.456")
    Then Result should contain approximately 123.456

  @python_e2e
  Scenario: should bind decimal with high precision
    Given Snowflake client is logged in with pyformat paramstyle
    When Query "SELECT %s::NUMBER(38,18)" is executed with high-precision Decimal
    Then Result should preserve full precision

  # =========================================================================== #
  #                   Literal percent in queries                               #
  # =========================================================================== #

  @python_e2e
  Scenario: should handle like with percent wildcard
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (name VARCHAR) exists
    And Rows ["Alice"], ["Bob"], ["Charlie"] are inserted
    When Query with LIKE '%%li%%' pattern is executed (escaped percent signs)
    Then Result should contain "Alice" and "Charlie"

  @python_e2e
  Scenario: should handle like with param and percent
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (name VARCHAR) exists
    And Rows ["Alice"], ["Bob"] are inserted
    When Query with LIKE %s is executed with parameter "%li%"
    Then Result should contain "Alice"

  # =========================================================================== #
  #                   List binding with special characters                     #
  # =========================================================================== #

  @python_e2e
  Scenario: should bind list with special chars in strings
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (name VARCHAR) exists
    And Rows with special characters are inserted
    When Query with WHERE name IN (%s) is executed with list containing special characters
    Then Result should contain the matching rows with preserved special characters

  # =========================================================================== #
  #                   Executemany rowcount                                     #
  # =========================================================================== #

  @python_e2e
  Scenario: should accumulate rowcount in executemany
    Given Snowflake client is logged in with pyformat paramstyle
    And A temporary table with columns (id NUMBER, name VARCHAR) exists
    When executemany is called to insert 3 rows using %s binding
    Then cursor.rowcount should be 3
