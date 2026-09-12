import unittest
from schema_lookups import lookup_arrays


class LookupTests(unittest.TestCase):
    def test_conditional_tables_keep_widest_choice_in_either_order(self):
        wide = 'table[] = {"RACE", "BEACON", "STATUS"};'
        narrow = 'table[] = {"RACE", "BEACON"};'
        for source in [wide + narrow, narrow + wide]:
            self.assertEqual(lookup_arrays(source)['table'], ['RACE', 'BEACON', 'STATUS'])

    def test_conflicting_ordinals_fail_instead_of_changing_defaults(self):
        with self.assertRaises(ValueError):
            lookup_arrays('table[] = {"A", "B"}; table[] = {"B", "A"};')

    def test_sized_table(self):
        self.assertEqual(lookup_arrays('table[COUNT] = {"A"};', sized=True), {'table': ['A']})
