import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('release', Path(__file__).resolve().parents[1]/'release.py')
release = importlib.util.module_from_spec(spec); spec.loader.exec_module(release)

class ReleaseContract(unittest.TestCase):
    def populate(self, folder, targets=release.PLATFORMS):
        for target in targets:
            for name in release.packages('0.2.0',target):
                (folder/name).write_bytes(name.encode())
                (folder/(name+'.sha256')).write_text(f'{release.checksum(folder/name)}  {name}\n')

    def test_tag_must_match_version(self):
        release.validate_tag('v0.2.0','0.2.0')
        with self.assertRaises(ValueError): release.validate_tag('v0.1.0','0.2.0')

    def test_manifest_requires_every_platform_and_contains_exact_urls(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            self.populate(root, ('macos-arm64',))
            with self.assertRaises(ValueError): release.manifest(root,'0.2.0','notes')
            self.populate(root)
            result=release.manifest(root,'0.2.0','notes')
            self.assertEqual(len(result['downloads']),7)
            for target in release.PLATFORMS:
                for name,key in release.packages('0.2.0',target).items():
                    self.assertEqual(result['downloads'][key]['url'],f'https://github.com/eyotang/RPCRad/releases/download/v0.2.0/{name}')
            self.assertEqual(result['platforms'],{})

    def test_manifest_rejects_tampered_or_unexpected_assets(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary); self.populate(root)
            name=release.asset_names('0.2.0')[0]
            (root/name).write_bytes(b'tampered')
            with self.assertRaises(ValueError): release.manifest(root,'0.2.0','')
            self.populate(root)
            (root/'private.key').write_text('never publish')
            with self.assertRaises(ValueError): release.manifest(root,'0.2.0','')

    def test_per_platform_validation_rejects_incomplete_or_extra_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary); self.populate(root, ('windows-x64',))
            release.validate_assets(root,'0.2.0',('windows-x64',))
            (root/release.asset_names('0.2.0',('windows-x64',))[0]).unlink()
            with self.assertRaises(ValueError): release.validate_assets(root,'0.2.0',('windows-x64',))
