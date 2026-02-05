// ABOUTME: Unit tests for form field validators (pure functions)
// ABOUTME: Tests required, minLength, maxLength, email, range, pattern validators

import { describe, it, expect } from 'vitest';
import {
  required,
  minLength,
  maxLength,
  email,
  range,
  pattern,
} from '../src/useFormField';

describe('required validator', () => {
  const validate = required();

  it('fails for empty string', () => {
    expect(validate('').valid).toBe(false);
  });

  it('fails for whitespace-only string', () => {
    expect(validate('   ').valid).toBe(false);
  });

  it('passes for non-empty string', () => {
    expect(validate('hello').valid).toBe(true);
  });

  it('uses custom error message', () => {
    const customValidator = required('Name is required');
    expect(customValidator('').message).toBe('Name is required');
  });
});

describe('minLength validator', () => {
  const validate = minLength(3);

  it('fails for strings shorter than min', () => {
    expect(validate('ab').valid).toBe(false);
  });

  it('passes for strings at min length', () => {
    expect(validate('abc').valid).toBe(true);
  });

  it('passes for strings longer than min', () => {
    expect(validate('abcdef').valid).toBe(true);
  });

  it('has default error message', () => {
    expect(validate('a').message).toBe('Must be at least 3 characters');
  });

  it('uses custom error message', () => {
    const custom = minLength(5, 'Too short!');
    expect(custom('ab').message).toBe('Too short!');
  });
});

describe('maxLength validator', () => {
  const validate = maxLength(5);

  it('passes for strings shorter than max', () => {
    expect(validate('abc').valid).toBe(true);
  });

  it('passes for strings at max length', () => {
    expect(validate('abcde').valid).toBe(true);
  });

  it('fails for strings longer than max', () => {
    expect(validate('abcdef').valid).toBe(false);
  });

  it('has default error message', () => {
    expect(validate('abcdef').message).toBe('Must be at most 5 characters');
  });
});

describe('email validator', () => {
  const validate = email();

  it('passes for valid email', () => {
    expect(validate('user@example.com').valid).toBe(true);
  });

  it('fails for email without @', () => {
    expect(validate('userexample.com').valid).toBe(false);
  });

  it('fails for email without domain', () => {
    expect(validate('user@').valid).toBe(false);
  });

  it('passes for empty string (not required)', () => {
    expect(validate('').valid).toBe(true);
  });

  it('uses custom error message', () => {
    const custom = email('Bad email');
    expect(custom('invalid').message).toBe('Bad email');
  });
});

describe('range validator', () => {
  const validate = range(1, 10);

  it('passes for value within range', () => {
    expect(validate(5).valid).toBe(true);
  });

  it('passes for value at min', () => {
    expect(validate(1).valid).toBe(true);
  });

  it('passes for value at max', () => {
    expect(validate(10).valid).toBe(true);
  });

  it('fails for value below min', () => {
    expect(validate(0).valid).toBe(false);
  });

  it('fails for value above max', () => {
    expect(validate(11).valid).toBe(false);
  });

  it('has default error message', () => {
    expect(validate(0).message).toBe('Must be between 1 and 10');
  });
});

describe('pattern validator', () => {
  const validate = pattern(/^\d{3}-\d{4}$/);

  it('passes for matching pattern', () => {
    expect(validate('123-4567').valid).toBe(true);
  });

  it('fails for non-matching pattern', () => {
    expect(validate('abc').valid).toBe(false);
  });

  it('passes for empty string (not required)', () => {
    expect(validate('').valid).toBe(true);
  });

  it('uses custom error message', () => {
    const custom = pattern(/^\d+$/, 'Numbers only');
    expect(custom('abc').message).toBe('Numbers only');
  });
});
