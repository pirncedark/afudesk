import { test } from 'node:test';
import { expect } from '@playwright/test';
import { existsSync } from 'node:fs';
import { mkdtemp, mkdir, symlink, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { parseSites } from '../src/config.js';
import { prepareProfile, manualLoginCommand } from '../src/profile.js';

const site = { id: 'demo', name: 'Demo', url: 'https://example.test', permissions: ['oku', 'giriş'], loggedIn: { selector: '#member' } };

test('validates the allowlist and all four permissions', () => {
  const sites = parseSites({ siteler: [{ ...site, permissions: ['oku', 'giriş', 'form-doldur', 'hassas'] }] });
  expect(sites).toHaveLength(1);
  expect(sites[0].permissions).toEqual(['oku', 'giriş', 'form-doldur', 'hassas']);
  expect(parseSites({ siteler: [] })).toEqual([]);
});

for (const [name, input] of Object.entries({
  'unknown permission': { siteler: [{ ...site, permissions: ['admin'] }] },
  'duplicate id': { siteler: [site, site] },
  'unsafe id': { siteler: [{ ...site, id: '../secret' }] },
  'public HTTP': { siteler: [{ ...site, url: 'http://example.test' }] },
  'credentials in URL': { siteler: [{ ...site, url: 'https://user:pass@example.test' }] },
  'missing session rule': { siteler: [{ ...site, loggedIn: {} }] },
  'extra secret field': { siteler: [{ ...site, password: 'fixture-only' }] },
  'foreign session URL': { siteler: [{ ...site, loggedIn: { url: 'https://foreign.test/home' } }] },
})) {
  test(`rejects ${name}`, () => { expect(() => parseSites(input)).toThrow(); });
}

test('accepts localhost mock URLs and rejects malformed selector settings', () => {
  expect(parseSites({ siteler: [{ ...site, url: 'http://127.0.0.1:1234' }] })).toHaveLength(1);
  expect(() => parseSites({ siteler: [{ ...site, accountSelector: '' }] })).toThrow();
});

test('creates Windows default profile outside the repository', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'afu-profile-'));
  const profile = await prepareProfile({ repoRoot: path.join(root, 'repo'), platform: 'win32', localAppData: root });
  expect(profile).toBe(path.join(root, 'AfuDesk', 'otomasyon', 'profil'));
  expect(existsSync(profile)).toBe(true);
});

test('creates Linux default profile and restricts permissions on Linux', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'afu-linux-'));
  const profile = await prepareProfile({ repoRoot: path.join(root, 'repo'), platform: 'linux', home: root });
  expect(profile).toBe(path.join(root, '.local/share/afudesk/otomasyon/profil'));
  expect(existsSync(profile)).toBe(true);
  if (process.platform !== 'win32') expect((await stat(profile)).mode & 0o777).toBe(0o700);
});

test('rejects repository paths including traversal and symlink aliases', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'afu-boundary-'));
  const repo = path.join(root, 'repo');
  await mkdir(repo);
  for (const profilePath of [repo, path.join(repo, 'profil'), path.join(root, 'outside', '..', 'repo', 'profil')]) {
    await expect(prepareProfile({ repoRoot: repo, profilePath })).rejects.toThrow();
  }
  const alias = path.join(root, 'alias');
  await symlink(repo, alias, process.platform === 'win32' ? 'junction' : 'dir');
  await expect(prepareProfile({ repoRoot: repo, profilePath: path.join(alias, 'new', 'profil') })).rejects.toThrow();
  expect(existsSync(path.join(repo, 'new'))).toBe(false);
});

test('manual login uses the same separate profile without automation flags', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'afu-manual-'));
  const options = { repoRoot: path.join(root, 'repo'), profilePath: path.join(root, 'profil') };
  const command = await manualLoginCommand(options);
  expect(command.executable).not.toBe('');
  expect(command.args).toContain(`--user-data-dir=${options.profilePath}`);
  expect(command.args).toContain('https://accounts.google.com/');
  expect(command.args.some(arg => /automation|remote-debugging|disable-blink/.test(arg))).toBe(false);
});
