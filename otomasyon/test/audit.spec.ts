import { test } from 'node:test';
import { expect } from '@playwright/test';
import { mkdtemp, readFile, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { AuditLog } from '../src/audit.js';
import type { Result } from '../src/agent.js';
import type { Site } from '../src/config.js';

const sites: Site[] = [{ id: 'demo', name: 'Demo', url: 'https://example.test', permissions: ['giriş'], loggedIn: { selector: '#member' } }];

test('audit stores only fixed metadata, never passwords, cookies or free-form errors', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'afu-audit-'));
  const log = new AuditLog(sites, root);
  const dangerous = {
    status: 'stopped', reason: 'captcha', needsUserApproval: true,
    message: 'private-password-fixture secret-cookie-fixture',
    password: 'private-password-fixture', cookie: 'secret-cookie-fixture',
    cookies: [{ name: 'session', value: 'cookie-value-fixture' }],
    nested: { token: 'private-token-fixture' }, url: 'https://example.test?code=oauth-code-fixture',
  };
  const file = await log.record('demo', dangerous as Result);
  expect(file).toBe(path.join(root, 'demo', `${new Date().toISOString().slice(0, 10)}.jsonl`));
  const raw = await readFile(file, 'utf8');
  const entry = JSON.parse(raw);
  expect(Object.keys(entry).sort()).toEqual(['method', 'needsUserApproval', 'reason', 'site', 'status', 'timestamp']);
  expect(entry).toMatchObject({ site: 'demo', status: 'stopped', reason: 'captcha', method: 'google', needsUserApproval: true });
  for (const secret of ['private-password-fixture', 'secret-cookie-fixture', 'cookie-value-fixture', 'private-token-fixture', 'oauth-code-fixture']) expect(raw).not.toContain(secret);
  if (process.platform !== 'win32') expect((await stat(file)).mode & 0o777).toBe(0o600);
});

test('appends JSONL and never uses an unlisted identifier as a path or log value', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'afu-audit-path-'));
  const log = new AuditLog(sites, root);
  const result: Result = { status: 'signed-in', message: 'Success', needsUserApproval: false };
  const file = await log.record('demo', result);
  await log.record('demo', result);
  expect((await readFile(file, 'utf8')).trim().split('\n')).toHaveLength(2);
  const unknown = await log.record('../private-password-fixture', { status: 'stopped', reason: 'unlisted-site', needsUserApproval: true, message: 'Stop' });
  expect(path.dirname(unknown)).toBe(path.join(root, 'unlisted'));
  expect(await readFile(unknown, 'utf8')).not.toContain('private-password-fixture');
});

test('rejects forged audit statuses and reasons instead of copying arbitrary values', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'afu-audit-invalid-'));
  const log = new AuditLog(sites, root);
  await expect(log.record('demo', { status: 'password-fixture' } as unknown as Result)).rejects.toThrow();
  await expect(log.record('demo', { status: 'stopped', reason: 'cookie-fixture' } as unknown as Result)).rejects.toThrow();
});
