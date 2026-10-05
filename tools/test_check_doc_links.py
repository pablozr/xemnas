import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "doc_links", Path(__file__).with_name("check-doc-links.py")
)
links = importlib.util.module_from_spec(spec)
spec.loader.exec_module(links)


class DocLinksTests(unittest.TestCase):
    def test_destinations(self):
        text = '''[file](a.md#heading) ![image](image.png)
[ref]: <space%20name.md> "Title"
[external](https://example.com) [mail](mailto:a@example.com) [anchor](#here)
```md
[example](missing.md)
```
~~~
[example](missing2.md)
~~~
`[inline](missing3.md)`
'''
        self.assertEqual([url for _, url in links.destinations(text)], [
            "a.md#heading", "image.png", "space%20name.md",
            "https://example.com", "mailto:a@example.com", "#here",
        ])

    def test_files_directories_and_clean_clone(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "docs").mkdir()
            (root / "docs" / "space name.md").write_text("", encoding="utf-8")
            (root / "ignored.md").write_text("", encoding="utf-8")
            source = root / "docs" / "index.md"
            source.write_text('''[encoded](space%20name.md?raw=1#x)
[dir](../docs/) [missing](missing.md) [ignored](../ignored.md)
[external](https://example.com) [anchor](#x)
[reference]: space%20name.md
![image](space%20name.md)
''', encoding="utf-8")
            count, errors = links.check_file(root, "docs/index.md", {
                "docs/index.md", "docs/space name.md",
            })
            self.assertEqual(count, 6)
            self.assertEqual(len(errors), 2)
            self.assertIn("missing.md", errors[0])
            self.assertIn("ignored.md", errors[1])

    def test_outside_repository(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "index.md").write_text("[outside](../outside.md)", encoding="utf-8")
            self.assertEqual(len(links.check_file(root, "index.md", {"index.md"})[1]), 1)

    def test_fences_and_external_links_do_not_check_targets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "index.md").write_text('''````markdown
[missing](missing.md)
```
[still fenced](also-missing.md)
````
[url](https://example.com/x) [mail](mailto:a@example.com)
[anchor](#here) [network](//example.com/x)
''', encoding="utf-8")
            self.assertEqual(links.check_file(root, "index.md", {"index.md"}), (0, []))


if __name__ == "__main__":
    unittest.main()
