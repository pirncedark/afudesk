import { test, before, after, describe } from 'node:test';
import { expect } from '@playwright/test';
import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import type { Browser, BrowserContext } from 'playwright';
import type { ChildProcess } from 'node:child_process';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { signIn } from '../src/agent.js';
import type { Site } from '../src/config.js';
import { testBrowser } from './browser.js';

describe('local browser integration', () => {
let server: Server;
let browser: Browser;
let child: ChildProcess;
let origin: string;
let blockedHits = 0;
const stopPages: Record<string, string> = {
  consent: '<h1>Allow access to your Google Account</h1><button>Allow</button>',
  'account-creation': '<h1>Create your account</h1><button>Create account</button>',
  captcha: '<h1>Verify you are human</h1><iframe title="reCAPTCHA" src="/captcha-frame"></iframe><button>Continue</button>',
  '2fa': '<h1>2-Step Verification</h1><input autocomplete="one-time-code"><button>Verify</button>',
  phone: '<h1>Verify your phone number</h1><input type="tel"><button>Verify</button>',
  payment: '<h1>Payment information</h1><input autocomplete="cc-number"><button>Pay</button>',
};

before(async () => {
  server = createServer((req, res) => {
    const url = new URL(req.url!, 'http://localhost');
    res.setHeader('Content-Type', 'text/html; charset=utf-8');
    const stop = url.searchParams.get('stop');
    const button = url.searchParams.get('button');
    const mode = url.searchParams.get('mode');
    const next = stop ? `/stop/${stop}` : '/member';
    let html = '';
    if (url.pathname === '/login') {
      const href = `/picker?next=${encodeURIComponent(next)}&mode=${mode ?? ''}`;
      const name = button === 'tr' ? 'Google ile giriş' : 'Sign in with Google';
      html = button === 'link' ? `<a aria-label="Google sign in" href="${href}">Go</a>`
        : button === 'custom' ? `<button id="google-custom" onclick="location.href='${href}'">Start</button>`
        : `<button ${mode === 'popup' ? `onclick="window.open('${href}')"` : `onclick="location.href='${href}'"`}>${name}</button>`;
    } else if (url.pathname === '/picker') {
      if (mode === 'needs-password') html = '<h1>Sign in to Google</h1><input type="password">';
      else if (mode === 'ambiguous') html = '<h1>Choose an account</h1><button class="account">One</button><button class="account">Two</button>';
      else html = `<h1>Choose an account</h1><button class="account" onclick="location.href='${url.searchParams.get('next')}'">Mock account</button>`;
    } else if (url.pathname === '/member') html = '<h1 id="member">Welcome</h1>';
    else if (url.pathname.startsWith('/stop/')) {
      html = stopPages[url.pathname.split('/')[2]] ?? '<h1>Continue</h1><div id="custom-stop">Review</div><button>OK</button>';
      html += '<script>document.addEventListener("click",()=>fetch("/unsafe-action"))</script>';
    } else if (url.pathname === '/redirect') { res.writeHead(302, { Location: 'http://localhost:65000/forbidden' }); res.end(); return; }
    else if (url.pathname === '/unsafe-action') { blockedHits++; }
    else if (url.pathname === '/slow') html = '<script>setTimeout(()=>document.body.innerHTML=\'<h1 id="member">Welcome</h1>\',250)</script>';
    else if (url.pathname === '/hidden') html = '<h1 id="member">Welcome</h1><div hidden>CAPTCHA Payment information</div>';
    else if (url.pathname === '/frame-stop') html = '<h1 id="member">Welcome</h1><iframe src="/stop/payment"></iframe>';
    else if (url.pathname === '/hidden-first-consent') html = '<h1 id="member">Welcome</h1><div class="requires-consent" hidden>Review</div><div class="requires-consent">Review</div>';
    else if (url.pathname === '/hidden-first-payment') html = '<h1 id="member">Welcome</h1><input autocomplete="cc-number" hidden><input autocomplete="cc-number">';
    else if (url.pathname === '/secondary-signup') html = '<h1 id="member">Welcome</h1><a href="/register">Sign up</a>';
    else if (url.pathname === '/missing') html = '<h1>Please sign in</h1>';
    res.end(`<!doctype html><html><body>${html}</body></html>`);
  });
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  origin = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  ({ browser, process: child } = await testBrowser());
});

after(async () => {
  await browser?.close();
  child?.kill();
  await new Promise<void>(resolve => server?.close(() => resolve()) ?? resolve());
});

function site(url = '/login'): Site {
  return { id: 'demo', name: 'Demo', url: `${origin}${url}`, permissions: ['oku', 'giriş'], loggedIn: { selector: '#member' }, accountSelector: '.account' };
}

async function run(target: Site, overrides: Partial<Parameters<typeof signIn>[0]> = {}) {
  const context = await browser.newContext({ serviceWorkers: 'block' });
  const external: string[] = [];
  // Fulfil nothing externally: a faulty implementation cannot contact Google.
  await context.route('**/*', route => {
    if (new URL(route.request().url()).origin !== origin) { external.push(route.request().url()); return route.abort(); }
    return route.continue();
  });
  try {
    const auditRoot = await mkdtemp(path.join(tmpdir(), 'afu-login-log-'));
    const result = await signIn({ context, sites: [target], siteId: target.id, timeoutMs: 1200, auditRoot, ...overrides });
    return { result, external, urls: context.pages().map(page => page.url()) };
  } finally { await context.close(); }
}

test('recognizes an existing signed-in session without clicking', async () => {
  const { result } = await run(site('/member'));
  expect(result.status).toBe('signed-in');
});

for (const variant of ['', '&button=tr', '&button=link', '&button=custom', '&mode=popup']) {
  test(`signs in through a local Google button and account chooser ${variant || 'English'}`, async () => {
    const target = site(`/login?x=1${variant}`);
    if (variant === '&button=custom') target.googleButtonSelector = '#google-custom';
    const { result, external } = await run(target);
    expect(result.status).toBe('signed-in');
    expect(external).toEqual([]);
  });
}

test('waits for delayed session rendering', async () => {
  expect((await run(site('/slow'))).result.status).toBe('signed-in');
});

for (const reason of Object.keys(stopPages)) {
  test(`stops and requests user approval at ${reason} before any action`, async () => {
    blockedHits = 0;
    const { result } = await run(site(`/login?stop=${reason}`));
    expect(result).toMatchObject({ status: 'stopped', reason, needsUserApproval: true });
    expect(blockedHits).toBe(0);
  });
}

test('custom stop rule and iframe payment take precedence over session success', async () => {
  const custom = site('/stop/custom');
  custom.stopSelectors = { consent: '#custom-stop' };
  expect((await run(custom)).result).toMatchObject({ reason: 'consent', needsUserApproval: true });
  expect((await run(site('/frame-stop'))).result).toMatchObject({ reason: 'payment', needsUserApproval: true });
});

test('ignores hidden stop text', async () => {
  expect((await run(site('/hidden'))).result.status).toBe('signed-in');
});

test('secondary signup navigation does not prevent session detection', async () => {
  expect((await run(site('/secondary-signup'))).result.status).toBe('signed-in');
});

test('visible safety elements still stop when their first match is hidden', async () => {
  const target = site('/hidden-first-consent');
  target.stopSelectors = { consent: '.requires-consent' };
  expect((await run(target)).result).toMatchObject({ reason: 'consent', needsUserApproval: true });
  expect((await run(site('/hidden-first-payment'))).result).toMatchObject({ reason: 'payment', needsUserApproval: true });
});

test('explicit read-only status check never clicks the login button', async () => {
  const target = site('/login?stop=payment');
  expect((await run(target, { permission: 'oku' })).result).toMatchObject({ reason: 'permission', needsUserApproval: true });
  expect((await run(site('/member'), { permission: 'oku' })).result.status).toBe('signed-in');
});

test('does not choose between multiple accounts or enter a password', async () => {
  for (const [mode, reason] of [['ambiguous', 'account-choice'], ['needs-password', 'manual-login']]) {
    expect((await run(site(`/login?mode=${mode}`))).result).toMatchObject({ status: 'stopped', reason, needsUserApproval: true });
  }
});

test('does not choose an account without an approved selector', async () => {
  const target = site();
  delete target.accountSelector;
  expect((await run(target)).result).toMatchObject({ reason: 'account-choice', needsUserApproval: true });
});

test('unlisted site stops before navigation', async () => {
  const { result, urls } = await run(site(), { siteId: 'unknown' });
  expect(result).toMatchObject({ reason: 'unlisted-site', needsUserApproval: true });
  expect(urls).toEqual([]);
});

test('read-only permission cannot initiate login and sensitive permission always stops', async () => {
  const target = site();
  target.permissions = ['oku'];
  expect((await run(target)).result).toMatchObject({ reason: 'permission', needsUserApproval: true });
  target.permissions = ['giriş', 'hassas'];
  expect((await run(target, { permission: 'hassas' })).result).toMatchObject({ reason: 'permission', needsUserApproval: true });
});

test('blocks a redirect outside the allowlist before sending its request', async () => {
  const { result, external } = await run(site('/redirect'));
  expect(result).toMatchObject({ reason: 'unlisted-site', needsUserApproval: true });
  expect(external).toEqual([]);
});

test('URL session rule is exact and still constrained to the site origin', async () => {
  const target = site('/member');
  target.loggedIn = { url: `${origin}/member` };
  expect((await run(target)).result.status).toBe('signed-in');
  target.url = `${origin}/missing`;
  expect((await run(target)).result).toMatchObject({ status: 'stopped', needsUserApproval: true });
});
});
