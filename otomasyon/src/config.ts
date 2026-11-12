export type Permission = 'oku' | 'giriş' | 'form-doldur' | 'hassas';
export type StopReason = 'consent' | 'account-creation' | 'captcha' | '2fa' | 'phone' | 'payment' | 'unlisted-site' | 'permission' | 'account-choice' | 'manual-login' | 'login-unavailable' | 'error';
export interface Site {
  id: string;
  name: string;
  url: string;
  permissions: Permission[];
  loggedIn: { selector?: string; url?: string };
  googleButtonSelector?: string;
  accountSelector?: string;
  stopSelectors?: Partial<Record<StopReason, string>>;
}
// Load relative to the compiled module without depending on the caller's cwd.
const schema = JSON.parse(readFileSync(new URL('../../siteler.schema.json', import.meta.url), 'utf8'));
const validate = new Ajv({ allErrors: true }).compile(schema);

function safeUrl(value: string): URL {
  const url = new URL(value);
  const local = ['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname);
  if (url.username || url.password || !(url.protocol === 'https:' || (local && url.protocol === 'http:'))) {
    throw new Error('Site adresi güvenli değil; izinli site ayarlarını düzeltin.');
  }
  return url;
}

export function parseSites(input: unknown): Site[] {
  if (!validate(input)) throw new Error('Site ayarları geçersiz; izinli site listesini düzeltin.');
  const sites = (input as { siteler: Site[] }).siteler;
  const ids = new Set<string>();
  for (const site of sites) {
    const url = safeUrl(site.url);
    if (ids.has(site.id) || (site.loggedIn.url && safeUrl(site.loggedIn.url).origin !== url.origin)) {
      throw new Error('Site ayarları çakışıyor; izinli site listesini düzeltin.');
    }
    ids.add(site.id);
  }
  return structuredClone(sites);
}

export async function loadSites(file: string): Promise<Site[]> {
  try { return parseSites(JSON.parse(readFileSync(file, 'utf8'))); }
  catch { throw new Error('Site listesi okunamadı; site ayarlarını kontrol edin.'); }
}
import { Ajv } from 'ajv';
import { readFileSync } from 'node:fs';

