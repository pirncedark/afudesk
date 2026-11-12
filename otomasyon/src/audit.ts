import type { Result } from './agent.js';
import { parseSites, type Site } from './config.js';
import { constants } from 'node:fs';
import { chmod, mkdir, open, realpath } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const reasons = new Set(['consent', 'account-creation', 'captcha', '2fa', 'phone', 'payment', 'unlisted-site', 'permission', 'account-choice', 'manual-login', 'login-unavailable', 'error']);

export class AuditLog {
  private readonly ids: Set<string>;
  private readonly root: string;
  constructor(sites: Site[], root = fileURLToPath(new URL('../../kayit/', import.meta.url))) {
    this.ids = new Set(parseSites({ siteler: sites }).map(site => site.id));
    this.root = path.resolve(root);
  }

  async record(siteId: string, result: Result): Promise<string> {
    try {
      if (!['signed-in', 'stopped'].includes(result.status) || (result.status === 'stopped' && !reasons.has(result.reason))) {
        throw new Error('Invalid audit event');
      }
      const timestamp = new Date().toISOString();
      const site = this.ids.has(siteId) ? siteId : 'unlisted';
      const directory = path.join(this.root, site);
      await mkdir(directory, { recursive: true, mode: 0o700 });
      const [realRoot, realDirectory] = await Promise.all([realpath(this.root), realpath(directory)]);
      const relative = path.relative(realRoot, realDirectory);
      if (relative.startsWith('..') || path.isAbsolute(relative)) throw new Error('Unsafe audit directory');
      if (process.platform !== 'win32') await chmod(directory, 0o700);
      const file = path.join(directory, `${timestamp.slice(0, 10)}.jsonl`);
      // Construct a fresh object: never serialize input messages, errors, URLs,
      // form fields, storage, cookies, or arbitrary nested metadata.
      const entry = {
        timestamp, site, method: 'google', status: result.status,
        ...(result.status === 'stopped' ? { reason: result.reason } : {}),
        needsUserApproval: result.status === 'stopped',
      };
      const handle = await open(file, constants.O_WRONLY | constants.O_CREAT | constants.O_APPEND | (constants.O_NOFOLLOW ?? 0), 0o600);
      try {
        if (process.platform !== 'win32') await handle.chmod(0o600);
        await handle.writeFile(`${JSON.stringify(entry)}\n`, 'utf8');
      } finally { await handle.close(); }
      return file;
    } catch { throw new Error('İşlem kaydı yazılamadı; kayıt klasörünün erişimini kontrol edin.'); }
  }
}
