"""Rewrite Ubuntu archive URIs on disposable CI runners, preserving trust keys."""
import pathlib
import re
import sys
from urllib.parse import urlsplit


def ubuntu_uri(uri):
    return (uri.startswith('mirror+file:/etc/apt/apt-mirrors')
            or urlsplit(uri).hostname in {
                'archive.ubuntu.com', 'azure.archive.ubuntu.com',
                'security.ubuntu.com', 'ports.ubuntu.com'})


def rewrite(text, mirror, deb822=False):
    if deb822:
        return re.sub(r'(?m)^(URIs:[ \t]*)(.*)$',
                      lambda m: m[1] + re.sub(r'\S+', lambda u: mirror if ubuntu_uri(u[0]) else u[0], m[2]), text)
    return re.sub(r'(?m)^(deb(?:-src)?\s+(?:\[[^\]]*\]\s+)?)(\S+)(\s+)',
                  lambda m: m[1] + (mirror if ubuntu_uri(m[2]) else m[2]) + m[3], text)


if __name__ == '__main__':
    for path in [pathlib.Path('/etc/apt/sources.list'), *pathlib.Path('/etc/apt/sources.list.d').glob('*.list'), *pathlib.Path('/etc/apt/sources.list.d').glob('*.sources')]:
        if path.exists():
            path.write_text(rewrite(path.read_text(), sys.argv[1], path.suffix == '.sources'))
