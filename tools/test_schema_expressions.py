import unittest

from schema_expressions import resolver


class SchemaExpressionsTest(unittest.TestCase):
    def test_signed_alias_arithmetic_and_enum_bounds(self):
        resolve = resolver(['''
            #define LIMIT 450
            #define COUNT 8U
            enum { FIRST = -1, SECOND, THIRD = COUNT };
        '''])
        self.assertEqual(resolve('-LIMIT'), -450)
        self.assertEqual(resolve('(1 << COUNT) - 1'), 255)
        self.assertEqual(resolve('SECOND'), 0)
        self.assertEqual(resolve('THIRD - 1'), 7)

    def test_conditional_unknown_and_executable_expressions_are_not_bounds(self):
        resolve = resolver(['''
            #define COUNT 3
            #define COUNT 1
            enum { FIRST,
            #ifdef FEATURE
                SECOND,
            #endif
                LAST };
        '''])
        for expression in ['COUNT', 'LAST', 'MISSING', "__import__('os')", 'object.value']:
            with self.subTest(expression=expression), self.assertRaises(ValueError):
                resolve(expression)


if __name__ == '__main__':
    unittest.main()
