// A single-process node:test entry point also works on Node 20 and restricted
// Windows hosts; Playwright still drives a real browser in integration tests.
import './config-profile.spec.js';
import './audit.spec.js';
import './safety-policy.spec.js';
import './login.spec.js';
import './cli.spec.js';
