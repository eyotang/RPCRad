import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from release_source import validate_source
from release import PLATFORMS, REPOSITORY


class ReleaseSource(unittest.TestCase):
    def setUp(self):
        self.run = dict(status='completed', event='push', path='.github/workflows/ci.yml',
                        head_branch='v0.2.0', head_sha='release-sha',
                        repository={'full_name': REPOSITORY}, head_repository={'full_name': REPOSITORY})
        self.jobs = [dict(name=f'build (runner, {p}, app)', conclusion='success') for p in PLATFORMS]

    def validate(self, **overrides):
        args = dict(run=self.run, jobs=self.jobs, tag='v0.2.0', revision='release-sha',
                    changed=['tools/wayland-smoke.py', '.github/workflows/ci.yml'])
        args.update(overrides)
        validate_source(**args)

    def test_only_verification_changes_can_reuse_successful_native_artifacts(self):
        self.validate()
        for path in ['src/main.rs', 'Cargo.lock', '.go-version', 'tools/release.py', 'assets/RPCRad.ico']:
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.validate(changed=[path])

    def test_missing_failed_or_duplicated_platform_is_rejected(self):
        with self.assertRaises(ValueError): self.validate(jobs=self.jobs[:-1])
        failed = copy.deepcopy(self.jobs); failed[0]['conclusion'] = 'failure'
        with self.assertRaises(ValueError): self.validate(jobs=failed)
        with self.assertRaises(ValueError): self.validate(jobs=self.jobs[:-1] + [self.jobs[0]])

    def test_wrong_tag_revision_repository_or_workflow_is_rejected(self):
        for key, value in [('head_sha', 'other'), ('head_branch', 'main'), ('event', 'pull_request'),
                           ('path', 'other.yml'), ('repository', {'full_name': 'other/repo'}),
                           ('head_repository', {'full_name': 'other/repo'}), ('status', 'in_progress')]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.validate(run={**self.run, key: value})
