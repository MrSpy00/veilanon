/**
 * VeilAnon E2E Test Harness — Assertions & Test Structuring
 * High-precision assertion helpers with informative failure diagnostics.
 */

export class AssertionError extends Error {
  constructor(message, actual, expected) {
    super(message);
    this.name = 'AssertionError';
    this.actual = actual;
    this.expected = expected;
  }
}

export function assert(condition, message = 'Assertion failed') {
  if (!condition) {
    throw new AssertionError(message, condition, true);
  }
}

export function assertEqual(actual, expected, message = '') {
  if (actual !== expected) {
    const msg = message ? `${message} — ` : '';
    throw new AssertionError(`${msg}Expected: ${JSON.stringify(expected)}, Actual: ${JSON.stringify(actual)}`, actual, expected);
  }
}

export function assertNotEqual(actual, unexpected, message = '') {
  if (actual === unexpected) {
    const msg = message ? `${message} — ` : '';
    throw new AssertionError(`${msg}Did not expect: ${JSON.stringify(unexpected)}, but received it`, actual, unexpected);
  }
}

export function assertDeepEqual(actual, expected, message = '') {
  const actualStr = JSON.stringify(actual);
  const expectedStr = JSON.stringify(expected);
  if (actualStr !== expectedStr) {
    const msg = message ? `${message} — ` : '';
    throw new AssertionError(`${msg}Objects not deeply equal.\nExpected: ${expectedStr}\nActual:   ${actualStr}`, actual, expected);
  }
}

export function assertIncludes(container, target, message = '') {
  const msg = message ? `${message} — ` : '';
  if (typeof container === 'string') {
    if (!container.includes(target)) {
      throw new AssertionError(`${msg}String does not include substring '${target}'`, container, target);
    }
  } else if (Array.isArray(container)) {
    if (!container.includes(target)) {
      throw new AssertionError(`${msg}Array does not include item '${target}'`, container, target);
    }
  } else {
    throw new AssertionError(`${msg}Target container must be a string or array`, container, target);
  }
}

export function assertMatch(str, regex, message = '') {
  const msg = message ? `${message} — ` : '';
  if (!regex.test(str)) {
    throw new AssertionError(`${msg}String '${str}' does not match pattern ${regex}`, str, regex);
  }
}

export async function assertThrowsAsync(fn, expectedErrOrRegex, message = '') {
  const msg = message ? `${message} — ` : '';
  let threw = false;
  let caughtErr = null;
  try {
    await fn();
  } catch (err) {
    threw = true;
    caughtErr = err;
  }

  if (!threw) {
    throw new AssertionError(`${msg}Expected function to throw an error, but it succeeded without error`, null, expectedErrOrRegex);
  }

  if (expectedErrOrRegex) {
    if (typeof expectedErrOrRegex === 'string') {
      if (!caughtErr.message.includes(expectedErrOrRegex)) {
        throw new AssertionError(`${msg}Error message '${caughtErr.message}' did not include '${expectedErrOrRegex}'`, caughtErr.message, expectedErrOrRegex);
      }
    } else if (expectedErrOrRegex instanceof RegExp) {
      if (!expectedErrOrRegex.test(caughtErr.message)) {
        throw new AssertionError(`${msg}Error message '${caughtErr.message}' did not match pattern ${expectedErrOrRegex}`, caughtErr.message, expectedErrOrRegex);
      }
    }
  }
}

export function assertGreaterThanOrEqual(actual, expected, message = '') {
  const msg = message ? `${message} — ` : '';
  if (actual < expected) {
    throw new AssertionError(`${msg}Expected ${actual} >= ${expected}`, actual, expected);
  }
}

export function assertLessThanOrEqual(actual, expected, message = '') {
  const msg = message ? `${message} — ` : '';
  if (actual > expected) {
    throw new AssertionError(`${msg}Expected ${actual} <= ${expected}`, actual, expected);
  }
}
