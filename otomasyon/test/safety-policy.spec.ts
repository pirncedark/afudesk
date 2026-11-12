import { test } from 'node:test';
import { expect } from '@playwright/test';
import { classifyStopText } from '../src/safety.js';

test('a secondary signup link does not block the login page', () => {
  expect(classifyStopText('https://example.test/login', 'Sign in\nSign up\nKayıt ol', 'Sign in')).toBeUndefined();
});

test('an active signup heading or signup destination still requires approval', () => {
  expect(classifyStopText('https://example.test/join', 'Create your account\nContinue', 'Create your account')).toBe('account-creation');
  expect(classifyStopText('https://example.test/register', 'Continue', 'Welcome')).toBe('account-creation');
});
