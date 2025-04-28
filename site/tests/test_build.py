"""Tests of the landing page build: python3 -m unittest discover -s site/tests -v"""
import json
import re
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "site"))
import build  # noqa: E402


def release():
    return json.loads((HERE / "release.json").read_text(encoding="utf-8"))


class ChangelogTest(unittest.TestCase):
    def test_sections(self):
        md = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
        s = build.changelog_sections(md, 5)
        self.assertEqual(s[-1]["version"], "0.1.0")
        self.assertIsNone(re.search(r"^## ", s[-1]["body"], re.M))

    def test_list_with_continuation(self):
        self.assertEqual(
            build.md_to_html("### Added\n\n- **Bold**: one\n  two\n- three\n"),
            "<h4>Added</h4>\n<ul><li><strong>Bold</strong>: one two</li><li>three</li></ul>",
        )

    def test_html_in_code_is_escaped(self):
        self.assertIn("<code>a &lt; b &amp;</code>", build.md_to_html("- `a < b &`\n"))

    def test_unreleased_versions_are_not_news(self):
        md = "## [0.3.0] — 2026-03-01\n\n- c\n\n## [0.2.0] — 2026-02-01\n\n- b\n"
        t = json.loads((ROOT / "site/i18n/en.json").read_text(encoding="utf-8"))
        out = build.news_html(md, t, "0.2.0")
        self.assertIn("0.2.0", out)
        self.assertNotIn("0.3.0", out)


class ReleaseTest(unittest.TestCase):
    def test_all_assets_are_mapped(self):
        a = build.release_assets(release())
        self.assertEqual(set(a), set(build.ASSET_KINDS))
        self.assertEqual(a["linux-deb"]["name"], "rigscope-0.1.0-linux-x86_64.deb")

    def test_a_missing_asset_stops_the_build(self):
        r = release()
        r["assets"] = [x for x in r["assets"] if not x["name"].endswith(".dmg")]
        with self.assertRaises(SystemExit) as e:
            build.release_assets(r)
        self.assertIn("macos-universal.dmg", str(e.exception))


class PageTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.out = Path(cls.tmp.name)
        build.build(release(), cls.out)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_both_languages_are_built(self):
        en = (self.out / "index.html").read_text(encoding="utf-8")
        ro = (self.out / "ro/index.html").read_text(encoding="utf-8")
        self.assertIn('<html lang="en">', en)
        self.assertIn('<html lang="ro">', ro)
        self.assertIn("Download", en)
        self.assertIn("Descarcă", ro)
        self.assertNotIn("{{", en + ro)

    def test_download_links_point_at_the_release(self):
        en = (self.out / "index.html").read_text(encoding="utf-8")
        self.assertIn("releases/download/v0.1.0/rigscope-0.1.0-linux-x86_64.deb", en)
        self.assertIn("SHA256SUMS.txt", en)
        self.assertIn("v0.1.0", en)

    def test_romanian_page_uses_relative_paths_one_level_up(self):
        ro = (self.out / "ro/index.html").read_text(encoding="utf-8")
        self.assertIn('href="../assets/style.css"', ro)
        self.assertIn('assets/memory-ro.png', ro)

    def test_every_local_reference_exists(self):
        for page in (self.out / "index.html", self.out / "ro/index.html"):
            html = page.read_text(encoding="utf-8")
            for ref in re.findall(r'(?:src|href)="((?:\.\./)?assets/[^"]+)"', html):
                self.assertTrue((page.parent / ref).exists(), f"{page.name}: {ref}")

    def test_no_external_requests(self):
        for page in (self.out / "index.html", self.out / "ro/index.html"):
            html = page.read_text(encoding="utf-8")
            for ref in re.findall(r'<(?:script|link|img)[^>]+(?:src|href)="(https?://[^"]+)"', html):
                self.assertTrue(ref.startswith("https://nazdravan.github.io/"), ref)
        css = (self.out / "assets/style.css").read_text(encoding="utf-8")
        self.assertNotIn("http", css)

    def test_theme_css_has_the_theme_colours(self):
        theme = tomllib.loads((ROOT / "assets/theme.toml").read_text(encoding="utf-8"))
        css = (self.out / "assets/theme.css").read_text(encoding="utf-8")
        for key in ("accent", "accent_press", "warn", "bg_light", "fg_light", "bg_dark", "fg_dark"):
            self.assertIn(theme[key], css, key)

    def test_style_has_no_colour_literals(self):
        css = (ROOT / "site/assets/style.css").read_text(encoding="utf-8")
        self.assertIsNone(re.search(r"#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(", css))

    def test_og_image_is_1200_by_630(self):
        data = (self.out / "assets/og.png").read_bytes()
        self.assertEqual(int.from_bytes(data[16:20], "big"), 1200)
        self.assertEqual(int.from_bytes(data[20:24], "big"), 630)


class I18nTest(unittest.TestCase):
    def test_languages_have_the_same_keys(self):
        en = json.loads((ROOT / "site/i18n/en.json").read_text(encoding="utf-8"))
        ro = json.loads((ROOT / "site/i18n/ro.json").read_text(encoding="utf-8"))
        self.assertEqual(set(en), set(ro))


if __name__ == "__main__":
    unittest.main()
