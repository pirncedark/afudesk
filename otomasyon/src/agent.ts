import type { BrowserContext, Locator, Page, Route } from 'playwright';
import { setTimeout } from 'node:timers/promises';
import { parseSites, type Permission, type Site, type StopReason } from './config.js';
import { detectStop } from './safety.js';
import { AuditLog } from './audit.js';
export type Result = { status: 'signed-in'; message: string; needsUserApproval: false } | { status: 'stopped'; reason: StopReason; message: string; needsUserApproval: true };
export interface LoginOptions { context: BrowserContext; sites: Site[]; siteId: string; permission?: Permission; timeoutMs?: number; auditRoot?: string }
const messages: Record<StopReason, string> = {
  consent: 'Yeni erişim izni gerekiyor; izin ekranını kendiniz onaylayın.',
  'account-creation': 'Yeni hesap oluşturulacak; devam etmek için kendiniz onaylayın.',
  captcha: 'İnsan doğrulaması gerekiyor; doğrulamayı kendiniz tamamlayın.',
  '2fa': 'İkinci doğrulama gerekiyor; doğrulamayı kendiniz tamamlayın.',
  phone: 'Telefon doğrulaması gerekiyor; doğrulamayı kendiniz tamamlayın.',
  payment: 'Ödeme bilgisi isteniyor; devam etmek için kendiniz onaylayın.',
  'unlisted-site': 'Bu siteye izin verilmemiş; devam etmek için siteyi onaylayın.',
  permission: 'Bu işlem için izin verilmemiş; devam etmek için işlem iznini onaylayın.',
  'account-choice': 'Google hesabı seçilmeli; kullanacağınız hesabı kendiniz seçin.',
  'manual-login': 'Google girişi gerekiyor; siteyi açıp kendiniz oturum açın.',
  'login-unavailable': 'Otomatik giriş tamamlanamadı; siteye kendiniz giriş yapın.',
  error: 'Giriş tamamlanamadı; siteyi açıp yeniden deneyin.',
};

export function stopped(reason: StopReason): Result {
  return { status: 'stopped', reason, needsUserApproval: true, message: messages[reason] };
}

async function singleVisible(locator: Locator): Promise<Locator | undefined> {
  const visible: Locator[] = [];
  for (const item of await locator.all()) if (await item.isVisible()) visible.push(item);
  return visible.length === 1 ? visible[0] : undefined;
}

export async function findGoogleButton(page: Page, site: Site): Promise<Locator | undefined> {
  if (site.googleButtonSelector) return singleVisible(page.locator(site.googleButtonSelector));
  const name = /sign in with google|google sign in|continue with google|google ile (?:giriş|devam)|google hesabıyla/i;
  return singleVisible(page.getByRole('button', { name }).or(page.getByRole('link', { name }))
    .or(page.locator('a[href^="https://accounts.google.com/"]')));
}

export async function isSignedIn(page: Page, site: Site): Promise<boolean> {
  if (new URL(page.url()).origin !== new URL(site.url).origin) return false;
  if (site.loggedIn.url && page.url() !== new URL(site.loggedIn.url).href) return false;
  if (site.loggedIn.selector && !await page.locator(site.loggedIn.selector).first().isVisible()) return false;
  return Boolean(site.loggedIn.url || site.loggedIn.selector);
}

/** Only login/read status is implemented; form submission and sensitive work
 * stay outside this module's first five steps. Use a dedicated context. */
async function performSignIn(options: LoginOptions): Promise<Result> {
  const context = options.context;
  let guard: ((route: Route) => Promise<void>) | undefined;
  let track: ((page: Page) => void) | undefined;
  let denied = false;
  try {
    const sites = parseSites({ siteler: options.sites });
    const site = sites.find(item => item.id === options.siteId);
    if (!site) return stopped('unlisted-site');
    const permission = options.permission ?? 'giriş';
    if (!site.permissions.includes(permission) || !['oku', 'giriş'].includes(permission)) return stopped('permission');
    const timeout = options.timeoutMs ?? 8000;
    if (!Number.isFinite(timeout) || timeout < 100 || timeout > 60_000) return stopped('error');
    const allowed = new Set([new URL(site.url).origin, 'https://accounts.google.com']);
    const pages = new Set<Page>();
    track = page => { pages.add(page); };
    context.on('page', track);
    guard = async route => {
      const request = route.request();
      if (!request.isNavigationRequest()) return route.fallback();
      if (!allowed.has(new URL(request.url()).origin)) {
        denied = true;
        return route.abort();
      }
      // The browser follows 302 hops inside its network stack without asking
      // route handlers again, so inspect the redirect target ourselves first.
      const response = await route.fetch({ maxRedirects: 0 });
      const location = response.headers()['location'];
      if (response.status() >= 300 && response.status() < 400 && location
        && !allowed.has(new URL(location, request.url()).origin)) {
        denied = true;
        return route.abort();
      }
      await route.fulfill({ response });
    };
    await context.route('**/*', guard);
    const page = await context.newPage();
    pages.add(page);
    await page.goto(site.url, { waitUntil: 'domcontentloaded', timeout });
    const deadline = Date.now() + timeout;
    let googleClicked = false;
    let accountClicked = false;
    while (Date.now() < deadline) {
      if (denied) return stopped('unlisted-site');
      const active = [...pages].filter(candidate => !candidate.isClosed() && candidate.url() !== 'about:blank');
      // All stop checks precede success and every automated action, including
      // popup/frame screens. A callback showing success cannot bypass consent.
      for (const candidate of active) {
        if (!allowed.has(new URL(candidate.url()).origin)) return stopped('unlisted-site');
        const reason = await detectStop(candidate, site);
        if (reason) return stopped(reason);
      }
      for (const candidate of active) if (await isSignedIn(candidate, site)) {
        return { status: 'signed-in', needsUserApproval: false, message: 'Siteye giriş yapıldı.' };
      }
      for (const candidate of active) {
        const choosing = await candidate.getByText(/choose an account|hesap seçin/i).first().isVisible();
        const accounts = site.accountSelector ? candidate.locator(site.accountSelector) : undefined;
        if (choosing || (accounts && await accounts.first().isVisible())) {
          if (permission !== 'giriş') return stopped('permission');
          if (!accounts) return stopped('account-choice');
          const account = await singleVisible(accounts);
          if (!account) return stopped('account-choice');
          if (!accountClicked) {
            // Recheck immediately before clicking; never accept consent or
            // challenge screens through an account-selector collision.
            const reason = await detectStop(candidate, site);
            if (reason) return stopped(reason);
            accountClicked = true;
            await account.click({ timeout: Math.max(100, deadline - Date.now()) });
          }
        } else if (!googleClicked) {
          const button = await findGoogleButton(candidate, site);
          if (button) {
            if (permission !== 'giriş') return stopped('permission');
            const reason = await detectStop(candidate, site);
            if (reason) return stopped(reason);
            googleClicked = true;
            await button.click({ timeout: Math.max(100, deadline - Date.now()) });
          }
        }
      }
      await setTimeout(75);
    }
    return stopped('login-unavailable');
  } catch {
    return stopped(denied ? 'unlisted-site' : 'error');
  } finally {
    if (guard) await context.unroute('**/*', guard);
    if (track) context.off('page', track);
  }
}

export async function signIn(options: LoginOptions): Promise<Result> {
  try {
    const audit = new AuditLog(options.sites, options.auditRoot);
    const result = await performSignIn(options);
    await audit.record(options.siteId, result);
    return result;
  } catch { return stopped('error'); }
}
