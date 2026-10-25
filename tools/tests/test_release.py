import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("release",Path(__file__).resolve().parents[1]/"release.py")
release = importlib.util.module_from_spec(spec); spec.loader.exec_module(release)

class ReleaseContract(unittest.TestCase):
    def test_tag_must_match_version(self):
        release.validate_tag("v0.1.0", "0.1.0")
        with self.assertRaises(ValueError): release.validate_tag("v0.2.0", "0.1.0")

    def test_manifest_rejects_tampering_and_extra_assets(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); name = release.asset_names("0.1.0")[0]
            (root/name).write_bytes(b"test-package")
            (root/(name+".sha256")).write_text(f"{release.checksum(root/name)}  {name}\n")
            result = release.manifest(root,"0.1.0","notes")
            self.assertEqual(result["platforms"],{})
            self.assertEqual(result["downloads"]["darwin-aarch64"]["url"],f"https://github.com/eyotang/RPCRad/releases/download/v0.1.0/{name}")
            (root/name).write_bytes(b"modified")
            with self.assertRaises(ValueError): release.manifest(root,"0.1.0","")
            (root/"private.key").write_text("must never be published")
            with self.assertRaises(ValueError): release.manifest(root,"0.1.0","")
