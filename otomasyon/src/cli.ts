import type { BrowserContext } from 'playwright';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { signIn, stopped, type Result } from './agent.js';
import { loadSites, type Permission, type Site, type StopReason } from './config.js';
import { openProfile } from './profile.js';
import { AuditLog } from './audit.js';

export interface CliDependencies {
  loadSites: () => Promise<Site[]>;
  openProfile: () => Promise<BrowserContext>;
  signIn: typeof signIn;
  record: (sites: Site[], siteId: string, result: Result) => Promise<void>;
}

const defaults: CliDependencies = {
  loadSites: () => loadSites(fileURLToPath(new URL('../../siteler.json', import.meta.url))),
  openProfile: () => openProfile({ repoRoot: fileURLToPath(new URL('../../../', import.meta.url)) }),
  signIn,
  record: async (sites, siteId, result) => { await new AuditLog(sites).record(siteId, result); },
};
const permissions = new Set<Permission>(['oku', 'giriş', 'form-doldur', 'hassas']);
const reasons = new Set<StopReason>([
  'consent', 'account-creation', 'captcha', '2fa', 'phone', 'payment',
  'unlisted-site', 'permission', 'account-choice', 'manual-login', 'login-unavailable', 'error',
]);

async function readRequest(input: AsyncIterable<Uint8Array | string>): Promise<{ siteId: string; permission: Permission }> {
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of input) {
    const bytes = Buffer.from(chunk);
    size += bytes.length;
    if (size > 8192) throw new Error('Invalid request');
    chunks.push(bytes);
  }
  const request = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(chunks)));
  if (!request || typeof request !== 'object' || Array.isArray(request)
    || Object.keys(request).length !== 2
    || typeof request.siteId !== 'string' || !/^[a-z0-9][a-z0-9-]{0,63}$/.test(request.siteId)
    || !permissions.has(request.permission)) throw new Error('Invalid request');
  return { siteId: request.siteId, permission: request.permission };
}

// Construct a fresh, fixed-vocabulary result. Never serialize exceptions,
// request data, browser objects, arbitrary messages or added result properties.
function publicResult(result: Result): Result {
  if (result.status === 'signed-in' && result.needsUserApproval === false) {
    return { status: 'signed-in', needsUserApproval: false, message: 'Siteye giriş yapıldı.' };
  }
  if (result.status === 'stopped' && result.needsUserApproval === true && reasons.has(result.reason)) {
    return stopped(result.reason);
  }
  return stopped('error');
}

/** One request until EOF, one JSON line; a stopped Result is a normal protocol
 * response (exit 0). The caller shows message and handles needsUserApproval. */
export async function runCli(
  input: AsyncIterable<Uint8Array | string>,
  output: { write: (line: string) => unknown },
  dependencies: CliDependencies = defaults,
): Promise<void> {
  let result: Result;
  try {
    const request = await readRequest(input);
    const sites = await dependencies.loadSites();
    const site = sites.find(candidate => candidate.id === request.siteId);
    if (!site || !site.permissions.includes(request.permission) || !['oku', 'giriş'].includes(request.permission)) {
      result = stopped(!site ? 'unlisted-site' : 'permission');
      await dependencies.record(sites, request.siteId, result);
    } else {
      const context = await dependencies.openProfile();
      try { result = publicResult(await dependencies.signIn({ context, sites, ...request })); }
      finally { await context.close(); }
    }
  } catch { result = stopped('error'); }
  output.write(`${JSON.stringify(result)}\n`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runCli(process.stdin, process.stdout);
}
