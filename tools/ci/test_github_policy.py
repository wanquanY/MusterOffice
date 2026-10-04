"""Additional bypass actors must never pass an integrity-policy audit."""
import unittest
from check_github import matches


class PolicyAuditTests(unittest.TestCase):
    def test_server_metadata_is_allowed_but_extra_bypass_is_not(self):
        policy = {'enforcement': 'active', 'bypass_actors': []}
        self.assertTrue(matches(policy, {**policy, 'id': 123}))
        self.assertFalse(matches(policy, {'enforcement': 'active', 'bypass_actors': [
            {'actor_type': 'RepositoryRole', 'actor_id': 5, 'bypass_mode': 'always'}]}))
        self.assertFalse(matches(policy, {**policy, 'enforcement': 'disabled'}))

    def test_reordered_rules_are_accepted_but_missing_rule_is_not(self):
        rules = [{'type': 'deletion'}, {'type': 'non_fast_forward'}]
        self.assertTrue(matches(rules, list(reversed(rules))))
        self.assertFalse(matches(rules, rules[:1]))


if __name__ == '__main__':
    unittest.main()
