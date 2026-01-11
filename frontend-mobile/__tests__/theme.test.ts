// ABOUTME: Unit tests for theme constants
// ABOUTME: Tests color values, spacing, and typography definitions

import { colors, spacing, fontSize, borderRadius } from '../src/constants/theme';

describe('Theme Constants', () => {
  describe('colors', () => {
    it('should have text colors', () => {
      expect(colors.text.primary).toBeDefined();
      expect(colors.text.secondary).toBeDefined();
      expect(colors.text.tertiary).toBeDefined();
    });

    it('should have text colors as strings (not objects)', () => {
      expect(typeof colors.text.primary).toBe('string');
      expect(typeof colors.text.secondary).toBe('string');
      expect(typeof colors.text.tertiary).toBe('string');
    });

    it('should have background colors', () => {
      expect(colors.background.primary).toBeDefined();
      expect(colors.background.secondary).toBeDefined();
      expect(colors.background.tertiary).toBeDefined();
    });

    it('should have primary color palette', () => {
      expect(colors.primary[400]).toBeDefined();
      expect(colors.primary[500]).toBeDefined();
      expect(colors.primary[600]).toBeDefined();
    });

    it('should have border colors', () => {
      expect(colors.border.default).toBeDefined();
      expect(colors.border.subtle).toBeDefined();
    });

    it('should have semantic colors', () => {
      expect(colors.success).toBeDefined();
      expect(colors.error).toBeDefined();
      expect(colors.warning).toBeDefined();
    });
  });

  describe('spacing', () => {
    it('should have spacing values', () => {
      expect(spacing.xs).toBeDefined();
      expect(spacing.sm).toBeDefined();
      expect(spacing.md).toBeDefined();
      expect(spacing.lg).toBeDefined();
      expect(spacing.xl).toBeDefined();
    });

    it('should have increasing spacing values', () => {
      expect(spacing.sm).toBeGreaterThan(spacing.xs);
      expect(spacing.md).toBeGreaterThan(spacing.sm);
      expect(spacing.lg).toBeGreaterThan(spacing.md);
      expect(spacing.xl).toBeGreaterThan(spacing.lg);
    });
  });

  describe('fontSize', () => {
    it('should have font size values', () => {
      expect(fontSize.xs).toBeDefined();
      expect(fontSize.sm).toBeDefined();
      expect(fontSize.md).toBeDefined();
      expect(fontSize.lg).toBeDefined();
      expect(fontSize.xl).toBeDefined();
    });

    it('should have increasing font sizes', () => {
      expect(fontSize.sm).toBeGreaterThan(fontSize.xs);
      expect(fontSize.md).toBeGreaterThan(fontSize.sm);
      expect(fontSize.lg).toBeGreaterThan(fontSize.md);
      expect(fontSize.xl).toBeGreaterThan(fontSize.lg);
    });
  });

  describe('borderRadius', () => {
    it('should have border radius values', () => {
      expect(borderRadius.sm).toBeDefined();
      expect(borderRadius.md).toBeDefined();
      expect(borderRadius.lg).toBeDefined();
    });
  });
});
