"""Platform dependency boundaries in the production SDK generator."""
import unittest
from build import production_dependencies, toml_document
import tomllib


class ProductionDependenciesTests(unittest.TestCase):
    def test_platform_only_dependencies_retain_their_predicate(self):
        manifest = {
            'dependencies': {'shared': {'workspace': True}},
            'dev-dependencies': {'test-only': {'workspace': True}},
            'target': {
                'cfg(unix)': {'dependencies': {'host': {'workspace': True}},
                              'dev-dependencies': {'unix-test': {'workspace': True}}},
                'cfg(windows)': {'dependencies': {'windows-host': {'workspace': True}}},
            },
        }
        dependencies = production_dependencies(manifest)
        self.assertEqual({name for name, _ in dependencies}, {'shared', 'host', 'windows-host'})
        self.assertEqual(set(manifest['dependencies']), {'shared'})
        self.assertEqual(set(manifest['target']['cfg(unix)']), {'dependencies'})
        self.assertNotIn('dev-dependencies', manifest)
        self.assertEqual(tomllib.loads(toml_document(manifest)), manifest)

    def test_target_build_scripts_and_unknown_sections_remain_forbidden(self):
        for section in ['build-dependencies', 'example', 'bin', 'features']:
            with self.subTest(section=section):
                with self.assertRaises(ValueError):
                    production_dependencies({'target': {'cfg(unix)': {section: {}}}})

    def test_target_development_only_dependency_is_not_promoted(self):
        manifest = {'target': {'cfg(unix)': {'dev-dependencies': {'test-only': {'workspace': True}}}}}
        self.assertEqual(production_dependencies(manifest), [])
        self.assertEqual(manifest['target']['cfg(unix)'], {})


if __name__ == '__main__':
    unittest.main()
