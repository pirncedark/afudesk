import { chromium, type BrowserContext } from 'playwright';
import { existsSync } from 'node:fs';
import { chmod, mkdir, realpath } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
export interface ProfileOptions { repoRoot: string; platform?: NodeJS.Platform; home?: string; localAppData?: string; profilePath?: string }
async function canonical(target: string): Promise<string> {
  try { return await realpath(target); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    const parent = path.dirname(target);
    if (parent === target) throw error;
    return path.join(await canonical(parent), path.basename(target));
  }
}

function contains(parent: string, child: string): boolean {
  const relative = path.relative(parent, child);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}

export async function prepareProfile(options: ProfileOptions): Promise<string> {
  try {
    const platform = options.platform ?? process.platform;
    const home = options.home ?? os.homedir();
    const profile = path.resolve(options.profilePath ?? (platform === 'win32'
      ? path.join(options.localAppData ?? process.env.LOCALAPPDATA ?? path.join(home, 'AppData', 'Local'), 'AfuDesk', 'otomasyon', 'profil')
      : path.join(home, '.local', 'share', 'afudesk', 'otomasyon', 'profil')));
    const repo = path.resolve(options.repoRoot);
    const [realRepo, realProfile] = await Promise.all([canonical(repo), canonical(profile)]);
    if (contains(repo, profile) || contains(realRepo, realProfile) || contains(realProfile, realRepo)) {
      throw new Error('Profile overlaps repository');
    }
    await mkdir(realProfile, { recursive: true, mode: 0o700 });
    if (process.platform !== 'win32') await chmod(realProfile, 0o700);
    return realProfile;
  } catch {
    throw new Error('Giriş profili güvenle açılamadı; depo dışında bir profil konumu seçin.');
  }
}

export async function openProfile(options: ProfileOptions & { headless?: boolean }): Promise<BrowserContext> {
  const profile = await prepareProfile(options);
  const launch = { headless: options.headless ?? false, serviceWorkers: 'block' as const, acceptDownloads: false };
  try { return await chromium.launchPersistentContext(profile, { ...launch, channel: 'chrome' }); }
  catch (error) {
    // Only a missing Chrome installation permits Linux's bundled fallback.
    if (process.platform === 'linux' && /not found|doesn't exist|not installed/i.test(String(error))) {
      try { return await chromium.launchPersistentContext(profile, launch); } catch { /* simple error below */ }
    }
    throw new Error('Tarayıcı açılamadı; Chrome kurulumunu ve açık giriş penceresini kontrol edin.');
  }
}

export async function manualLoginCommand(options: ProfileOptions): Promise<{ executable: string; args: string[] }> {
  const profile = await prepareProfile(options);
  const candidates = process.platform === 'win32' ? [
    path.join(process.env.PROGRAMFILES ?? 'C:\\Program Files', 'Google/Chrome/Application/chrome.exe'),
    path.join(process.env['PROGRAMFILES(X86)'] ?? 'C:\\Program Files (x86)', 'Google/Chrome/Application/chrome.exe'),
    path.join(process.env.LOCALAPPDATA ?? '', 'Google/Chrome/Application/chrome.exe'),
  ] : (process.env.PATH ?? '').split(path.delimiter).flatMap(dir => ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser'].map(name => path.join(dir, name)));
  const executable = candidates.find(file => existsSync(file)) ?? (process.platform === 'linux' && existsSync(chromium.executablePath()) ? chromium.executablePath() : undefined);
  if (!executable) throw new Error('Tarayıcı bulunamadı; Google Chrome kurun.');
  return { executable, args: [`--user-data-dir=${profile}`, '--new-window', 'https://accounts.google.com/'] };
}
