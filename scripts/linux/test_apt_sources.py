import unittest
from apt_sources import rewrite

MIRROR = 'https://archive.ubuntu.com/ubuntu'


class SourcesTest(unittest.TestCase):
    def test_microsoft_list_is_untouched(self):
        text = 'deb [arch=amd64 signed-by=/usr/share/keyrings/microsoft-prod.gpg] https://packages.microsoft.com/ubuntu/24.04/prod noble main\n'
        self.assertEqual(rewrite(text, MIRROR), text)

    def test_third_party_deb822_is_untouched(self):
        text = 'Types: deb\nURIs: https://packages.microsoft.com/ubuntu/24.04/prod\nSuites: noble\nSigned-By: /usr/share/keyrings/microsoft-prod.gpg\n'
        self.assertEqual(rewrite(text, MIRROR, True), text)

    def test_ubuntu_deb822_preserves_signing_key_and_suites(self):
        text = 'Types: deb\nURIs: mirror+file:/etc/apt/apt-mirrors.txt\nSuites: noble noble-updates noble-security\nComponents: main universe\nSigned-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n'
        self.assertEqual(rewrite(text, MIRROR, True), text.replace('mirror+file:/etc/apt/apt-mirrors.txt', MIRROR))

    def test_archive_list_changes_only_uri(self):
        text = 'deb [signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] http://azure.archive.ubuntu.com/ubuntu/ noble main\n# ubuntu notes\n'
        self.assertEqual(rewrite(text, MIRROR), text.replace('http://azure.archive.ubuntu.com/ubuntu/', MIRROR))


if __name__ == '__main__':
    unittest.main()
