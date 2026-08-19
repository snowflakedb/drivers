const assert = require('assert');
const ResultTestCommon = require('./result_test_common');
const RowMode = require('./../../../../lib/constants/row_mode');

// TODO:
// The "create unique names for duplicated column names" describe block that used to live
// here (testing unique_column_name_creator.js's row-key-rename algorithm) has been migrated
// to nodejs/tests/unit/column-names.test.ts and removed from this file.
//
// This remaining block is *not* migrated yet: it asserts that Column.getName() reflects
// renamed columns for object_with_renamed_duplicated_columns row mode, which the new driver
// does not implement (tracked as BD#1 in nodejs/BehaviorDifferences.yaml -- renameDuplicateColumnNames
// only renames row keys, not Column metadata). Once BD#1 is resolved, translate this block
// into a new-driver unit/e2e test and delete this file per the migration README's plan.
describe('Unique column names', function () {
  describe('result contains renamed columns depend on row mode', function () {
    const columnNames = [
      'KEY',
      'FOO',
      'KEY_1',
      'FOO',
      'KEY_3',
      'FOO_3',
      'KEY',
      'FOO',
      'KEY',
      'FOO',
    ];
    const testCases = [
      {
        title:
          'should return renamed columns for duplicates if ro mode object_with_renamed_duplicated_columns',
        rowMode: RowMode.OBJECT_WITH_RENAMED_DUPLICATED_COLUMNS,
        expectedColumnNames: [
          'KEY',
          'FOO',
          'KEY_1',
          'FOO_2',
          'KEY_3',
          'FOO_3',
          'KEY_2',
          'FOO_4',
          'KEY_4',
          'FOO_5',
        ],
      },
      {
        title: 'should not rename if row mode object',
        rowMode: RowMode.OBJECT,
        expectedColumnNames: columnNames,
      },
      {
        title: 'should not rename if row mode array',
        rowMode: RowMode.ARRAY,
        expectedColumnNames: columnNames,
      },
    ];
    const responseWithColumns = (columnRowSet) => {
      return {
        data: {
          parameters: [],
          rowtype: columnRowSet,
          rowset: [[]],
          total: 1,
          returned: 1,
        },
      };
    };

    testCases.forEach(({ title, rowMode, expectedColumnNames }) => {
      it(title, function (done) {
        const response = responseWithColumns(
          columnNames.map((columnName) => {
            return { name: columnName };
          }),
        );
        const resultOptions = ResultTestCommon.createResultOptions(response);
        resultOptions['rowMode'] = rowMode;

        ResultTestCommon.testResult(
          resultOptions,
          function each() {},
          function end(result) {
            const columnNames = result.getColumns().map((col) => col.getName());
            assert.deepStrictEqual(columnNames, expectedColumnNames);
            done();
          },
        );
      });
    });
  });
});
