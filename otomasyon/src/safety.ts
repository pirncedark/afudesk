import type { Locator, Page } from 'playwright';
import type { Site, StopReason } from './config.js';

const rules: { reason: StopReason; text: RegExp; selector?: string; url?: RegExp }[] = [
  { reason: 'payment', text: /payment information|payment details|add (?:a )?(?:credit |debit )?card|checkout|ödeme (?:bilgileri|yap)|kart bilgileri/i, selector: 'input[autocomplete^="cc-"], input[name="cardnumber"]', url: /\/(?:checkout|payment)(?:[/?#]|$)/i },
  { reason: 'captcha', text: /captcha|verify (?:that )?you are human|i.m not a robot|robot olmadığınızı|insan olduğunuzu/i, selector: 'iframe[src*="recaptcha"], iframe[src*="hcaptcha"], [data-sitekey]', url: /recaptcha|hcaptcha|\/captcha/i },
  { reason: '2fa', text: /2.step verification|two.factor|two.step|verification code|authenticator|iki adımlı|iki aşamalı|doğrulama kodu/i, selector: 'input[autocomplete="one-time-code"]', url: /\/challenge\/(?:totp|ootp|authzen|dp|sk)(?:[/?#]|$)/i },
  { reason: 'phone', text: /verify your phone|phone verification|phone number|telefon(?:unuzu)? doğrula|telefon numara/i, selector: 'input[type="tel"]', url: /\/challenge\/(?:ipp|sms)(?:[/?#]|$)/i },
  { reason: 'consent', text: /allow access|wants access|permissions requested|review permissions|consent|erişim izni|izin ver|izinleri (?:incele|gözden)/i, url: /\/consent(?:[/?#]|$)/i },
  { reason: 'account-creation', text: /create (?:your |an? )?account|sign up|hesap oluştur|üyelik oluştur|kayıt ol/i, url: /\/(?:signup|register)(?:[/?#]|$)/i },
  { reason: 'manual-login', text: /enter your password|şifrenizi girin/i, selector: 'input[type="password"]' },
];

export function classifyStopText(url: string, text: string, headings: string): StopReason | undefined {
  return rules.find(rule => rule.url?.test(url) || rule.text.test(rule.reason === 'account-creation' ? headings : text))?.reason;
}

async function anyVisible(locator: Locator): Promise<boolean> {
  for (const item of await locator.all()) if (await item.isVisible()) return true;
  return false;
}

export async function detectStop(page: Page, site: Site): Promise<StopReason | undefined> {
  for (const frame of page.frames()) {
    if (frame !== page.mainFrame()) {
      const element = await frame.frameElement();
      const visible = await element.isVisible();
      await element.dispose();
      if (!visible) continue;
    }
    for (const [reason, selector] of Object.entries(site.stopSelectors ?? {})) {
      if (await anyVisible(frame.locator(selector))) return reason as StopReason;
    }
    const text = await frame.locator('body').innerText({ timeout: 1000 });
    const headings: string[] = [];
    for (const heading of await frame.locator('h1, h2, [role="heading"], form legend').all()) {
      if (await heading.isVisible()) headings.push(await heading.innerText());
    }
    const textReason = classifyStopText(frame.url(), text, headings.join('\n'));
    if (textReason) return textReason;
    for (const rule of rules) {
      if (rule.selector && await anyVisible(frame.locator(rule.selector))) return rule.reason;
    }
  }
  return undefined;
}
