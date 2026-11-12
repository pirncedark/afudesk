import { chromium, type Browser } from 'playwright';
import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdtemp, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout } from 'node:timers/promises';

// CI-only --no-sandbox: Ubuntu 24.04 runners block unprivileged user namespaces.
// Chrome's CDP transport avoids Windows named-pipe restrictions in managed
// hosts. This is test-only: production uses launchPersistentContext.
export async function testBrowser(): Promise<{ browser: Browser; process: ChildProcess }> {
  const profile = await mkdtemp(path.join(tmpdir(), 'afu-mock-browser-'));
  const executable = existsSync(chromium.executablePath()) ? chromium.executablePath()
    : path.join(process.env.PROGRAMFILES ?? 'C:\\Program Files', 'Google/Chrome/Application/chrome.exe');
  const child = spawn(executable, [
    '--headless', ...(process.platform === 'linux' && process.env.CI ? ['--no-sandbox'] : []), '--no-first-run', '--no-default-browser-check', '--disable-background-networking',
    '--disable-component-update', '--disable-sync', '--remote-debugging-address=127.0.0.1',
    '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank',
  ], { stdio: ['ignore', 'ignore', 'inherit'], windowsHide: true });
  let failure: Error | undefined;
  child.on('error', error => { failure = error; });
  try {
    for (let i = 0; i < 100; i++) {
      if (failure) throw failure;
      if (child.exitCode !== null) throw new Error('Test browser exited');
      const port = await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8').catch(() => '');
      if (port) return { browser: await chromium.connectOverCDP(`http://127.0.0.1:${port.split('\n')[0]}`), process: child };
      await setTimeout(100);
    }
    throw new Error('Test browser did not start');
  } catch (error) { child.kill(); throw error; }
}
