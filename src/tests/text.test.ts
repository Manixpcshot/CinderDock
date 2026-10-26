import { describe, expect, it } from 'vitest';
import { applyTextMode, latinOnly, transliteratePersian } from '../lib/text';
import { createTranslator } from '../lib/i18n';
import type { Preferences } from '../lib/types';

const preferences: Preferences = {
  theme: 'ember-dark',
  uiLanguage: 'fa',
  textMode: 'native',
  fontScale: 1,
  desktopToolsEnabled: false,
  webAccessEnabled: true,
  maxToolIterations: 8,
  maxRetries: 2,
  terminalVisible: false,
  defaultModelId: null,
};

describe('Persian display modes', () => {
  it('keeps native text untouched', () => {
    expect(applyTextMode('سلام CinderDock ۱۲۳', 'native')).toBe('سلام CinderDock ۱۲۳');
  });

  it('transliterates Persian letters and digits for Finglish', () => {
    expect(transliteratePersian('سلام ۱۲۳')).toBe('slam 123');
    expect(applyTextMode('پروژه', 'finglish')).toBe('prvzhh');
  });

  it('keeps only Latin/ASCII characters in Latin mode', () => {
    expect(latinOnly('سلام، CinderDock! ۱۲۳')).toBe('slam, CinderDock! 123');
    expect(/[\u0600-\u06FF]/.test(latinOnly('پروژه测试'))).toBe(false);
  });

  it('uses English copy and LTR direction in Latin mode', () => {
    const translator = createTranslator({ ...preferences, textMode: 'latin' });
    expect(translator.t('newChat')).toBe('New chat');
    expect(translator.dir).toBe('ltr');
  });

  it('uses Persian copy and RTL direction in native mode', () => {
    const translator = createTranslator(preferences);
    expect(translator.t('newChat')).toBe('گفتوگوی جدید');
    expect(translator.dir).toBe('rtl');
  });
});
