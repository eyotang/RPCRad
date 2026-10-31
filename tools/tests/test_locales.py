"""Check shipped translations, interpolation contracts and README navigation."""
import collections
import json
from pathlib import Path
import re
import string
import unittest

ROOT = Path(__file__).resolve().parents[2]
LANGUAGES = ('zh-CN', 'en', 'ja', 'ko', 'fr', 'de', 'ru')
LITERAL = r'"(?:\\.|[^"\\])*"'


class Translations(unittest.TestCase):
    def test_catalogs_cover_every_application_and_builtin_plugin_message(self):
        catalogs = {code: json.loads((ROOT/'locales'/f'{code}.json').read_text(encoding='utf-8')) for code in LANGUAGES}
        required = set()
        for source in (ROOT/'src').rglob('*.rs'):
            for match in re.finditer(r'\btr(?:_format)?!\(\s*(' + LITERAL + ')', source.read_text(encoding='utf-8')):
                required.add(json.loads(match[1]))
        for source in (ROOT/'plugins').glob('*/plugin.json'):
            manifest = json.loads(source.read_text(encoding='utf-8'))
            required.update([manifest['name_en'], manifest['description_en']])
            required.update(field['label_en'] for field in manifest['fields'] if field['label_en'] != field['label'])
        for source in (ROOT/'plugins').glob('*/main.rhai'):
            for match in re.finditer(r'ui_text\(' + LITERAL + r',\s*(' + LITERAL + r')\)', source.read_text(encoding='utf-8')):
                required.add(json.loads(match[1]))
        for code, catalog in catalogs.items():
            with self.subTest(language=code):
                self.assertEqual(set(catalog), set(catalogs['en']))
                self.assertFalse(required-set(catalog))
                self.assertTrue(all(value.strip() for value in catalog.values()))

    def test_all_translated_formats_preserve_parameters_and_variable_examples(self):
        formats = json.loads((ROOT/'locales/formats.json').read_text(encoding='utf-8'))
        formatter = string.Formatter()
        for code in LANGUAGES:
            catalog = json.loads((ROOT/'locales'/f'{code}.json').read_text(encoding='utf-8'))
            for key in formats:
                with self.subTest(language=code, message=key):
                    fields = lambda text: collections.Counter((name, spec, conversion) for _, name, spec, conversion in formatter.parse(text) if name is not None)
                    self.assertEqual(fields(key), fields(catalog[key]))
            if code != 'zh-CN':
                for key, value in catalog.items():
                    if key not in formats:
                        self.assertEqual(re.findall(r'\{\{[^{}]+\}\}', key), re.findall(r'\{\{[^{}]+\}\}', value))

    def test_readmes_link_to_all_languages_and_valid_local_documents(self):
        files = ['README.md'] + [f'README.{code}.md' for code in LANGUAGES if code != 'zh-CN']
        for name in files:
            text = (ROOT/name).read_text(encoding='utf-8')
            with self.subTest(readme=name):
                for other in files:
                    self.assertIn(f']({other})', text)
                for link in re.findall(r'\]\(([^)]+)\)', text):
                    if '://' not in link and not link.startswith('#'):
                        self.assertTrue((ROOT/link.split('#')[0]).exists(), link)
