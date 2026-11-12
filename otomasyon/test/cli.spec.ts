import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { Readable } from 'node:stream';
import { fileURLToPath } from 'node:url';
import type { BrowserContext } from 'playwright';
import type { Result } from '../src/agent.js';
import type { Site } from '../src/config.js';
import { runCli, type CliDependencies } from '../src/cli.js';

const entry = fileURLToPath(new URL('../src/cli.js', import.meta.url));
const success: Result = { status: 'signed-in', needsUserApproval: false, message: 'Siteye giriş yapıldı.' };
const site: Site = { id: 'demo', name: 'Demo', url: 'https://example.test', permissions: ['oku', 'giriş'], loggedIn: { selector: '#member' } };

test('CLI executable writes one Result line for a denied site, without echoing stdin', async () => {
  const child = spawn(process.execPath, [entry], { cwd: process.env.TEMP ?? '/tmp', windowsHide: true });
  let stdout = '';
  let stderr = '';
  child.stdout.setEncoding('utf8').on('data', chunk => { stdout += chunk; });
  child.stderr.setEncoding('utf8').on('data', chunk => { stderr += chunk; });
  const completion = new Promise<number | null>((resolve, reject) => {
    child.on('error', reject);
    child.on('close', resolve);
  });
  // Even if this identifier is ever approved, sensitive work cannot open a
  // browser, so this executable test can never attempt a live Google login.
  child.stdin.end(JSON.stringify({ siteId: 'not-approved', permission: 'hassas' }));
  assert.equal(await completion, 0);
  assert.match(stdout, /^\{[^\n]+\}\n$/);
  assert.equal(JSON.parse(stdout).reason, 'unlisted-site');
  assert.equal(stderr, '');
});

async function harness(chunks: (string | Uint8Array)[], overrides: Partial<CliDependencies> = {}) {
  let output = '';
  let opened = 0;
  let closed = 0;
  const requests: unknown[] = [];
  const records: unknown[] = [];
  await runCli(Readable.from(chunks), { write: (text: string) => { output += text; return true; } }, {
    loadSites: async () => [site],
    openProfile: async () => {
      opened++;
      return { close: async () => { closed++; } } as unknown as BrowserContext;
    },
    signIn: async options => { requests.push(options); return success; },
    record: async (...args: unknown[]) => { records.push(args); },
    ...overrides,
  });
  assert.match(output, /^\{[^\n]+\}\n$/);
  return { result: JSON.parse(output), output, opened, closed, requests, records };
}

test('CLI forwards a chunked UTF-8 request and closes its browser before responding', async () => {
  const run = await harness(['{"siteId":"demo",', '"permission":"giriş"}\n']);
  assert.deepEqual(run.result, success);
  assert.equal(run.opened, 1);
  assert.equal(run.closed, 1);
  assert.equal(run.requests.length, 1);
  assert.equal((run.requests[0] as { permission: string }).permission, 'giriş');
  assert.equal((run.requests[0] as { siteId: string }).siteId, 'demo');
});

for (const permission of ['oku', 'giriş']) {
  test(`CLI preserves explicit ${permission} permission`, async () => {
    const run = await harness([JSON.stringify({ siteId: 'demo', permission })]);
    assert.equal((run.requests[0] as { permission: string }).permission, permission);
  });
}

for (const input of ['', 'invalid secret=fixture', 'null', '[]', '{}',
  '{"siteId":"../fixture","permission":"giriş"}',
  '{"siteId":"demo"}', '{"siteId":"demo","permission":"admin"}',
  '{"siteId":"demo","permission":"giriş","password":"fixture-secret"}',
  '{"siteId":"demo","permission":"giriş"}\n{"siteId":"demo","permission":"giriş"}',
  ' '.repeat(8193)]) {
  test(`CLI rejects malformed or oversized input (${input.length} bytes, ${input.slice(0, 12)})`, async () => {
    const run = await harness([input]);
    assert.equal(run.result.reason, 'error');
    assert.equal(run.opened, 0);
    assert.equal(run.requests.length, 0);
    assert.doesNotMatch(run.output, /fixture|password|secret=/);
  });
}

for (const [siteId, permission, reason] of [
  ['unknown', 'giriş', 'unlisted-site'], ['demo', 'form-doldur', 'permission'], ['demo', 'hassas', 'permission'],
]) {
  test(`CLI stops ${reason} before opening a profile (${permission})`, async () => {
    const run = await harness([JSON.stringify({ siteId, permission })]);
    assert.equal(run.result.reason, reason);
    assert.equal(run.opened, 0);
    assert.equal(run.records.length, 1);
  });
}

test('CLI denies a login permission missing from the allowlist', async () => {
  const run = await harness(['{"siteId":"demo","permission":"giriş"}'], {
    loadSites: async () => [{ ...site, permissions: ['oku'] }],
  });
  assert.equal(run.result.reason, 'permission');
  assert.equal(run.opened, 0);
});

for (const boundary of ['loadSites', 'openProfile', 'signIn', 'close', 'record']) {
  test(`CLI sanitizes ${boundary} errors and releases an opened browser`, async () => {
    const fail = async () => { throw new Error('password=fixture-secret cookie=fixture-cookie'); };
    const overrides = boundary === 'close' ? { openProfile: async () => ({ close: fail }) as unknown as BrowserContext }
      : { [boundary]: fail };
    const run = await harness([JSON.stringify({ siteId: boundary === 'record' ? 'unknown' : 'demo', permission: 'giriş' })], overrides);
    assert.equal(run.result.reason, 'error');
    assert.doesNotMatch(run.output, /fixture|password|cookie/);
    if (boundary === 'signIn') assert.equal(run.closed, 1);
  });
}

test('CLI reconstructs stopped results without arbitrary messages or extra fields', async () => {
  const run = await harness(['{"siteId":"demo","permission":"giriş"}'], {
    signIn: async () => ({ status: 'stopped', reason: 'captcha', needsUserApproval: true, message: 'fixture-secret', cookie: 'fixture-cookie' } as Result),
  });
  assert.equal(run.result.reason, 'captcha');
  assert.equal(run.result.needsUserApproval, true);
  assert.doesNotMatch(run.output, /fixture|cookie/);
});

test('CLI decodes a multi-byte permission split across raw stdin chunks', async () => {
  const bytes = Buffer.from('{"siteId":"demo","permission":"giriş"}');
  const split = bytes.indexOf(Buffer.from('ş')) + 1;
  const run = await harness([bytes.subarray(0, split), bytes.subarray(split)]);
  assert.equal(run.result.status, 'signed-in');
  assert.equal((run.requests[0] as { permission: string }).permission, 'giriş');
});

test('CLI rejects invalid UTF-8 without attempting a login', async () => {
  const run = await harness([Uint8Array.of(0xff)]);
  assert.equal(run.result.reason, 'error');
  assert.equal(run.opened, 0);
});

test('CLI suppresses failures while reading stdin', async () => {
  let output = '';
  const input = (async function* () { yield '{'; throw new Error('fixture-secret'); })();
  await runCli(input, { write: line => { output += line; } });
  assert.equal(JSON.parse(output).reason, 'error');
  assert.doesNotMatch(output, /fixture/);
});
