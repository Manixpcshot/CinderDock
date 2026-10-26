import type { TextMode } from './types';

const PERSIAN_DIGITS: Record<string, string> = {
  '۰': '0',
  '۱': '1',
  '۲': '2',
  '۳': '3',
  '۴': '4',
  '۵': '5',
  '۶': '6',
  '۷': '7',
  '۸': '8',
  '۹': '9',
  '٠': '0',
  '١': '1',
  '٢': '2',
  '٣': '3',
  '٤': '4',
  '٥': '5',
  '٦': '6',
  '٧': '7',
  '٨': '8',
  '٩': '9',
};

const LETTERS: Record<string, string> = {
  'آ': 'a',
  'ا': 'a',
  'أ': 'a',
  'إ': 'a',
  'ب': 'b',
  'پ': 'p',
  'ت': 't',
  'ث': 's',
  'ج': 'j',
  'چ': 'ch',
  'ح': 'h',
  'خ': 'kh',
  'د': 'd',
  'ذ': 'z',
  'ر': 'r',
  'ز': 'z',
  'ژ': 'zh',
  'س': 's',
  'ش': 'sh',
  'ص': 's',
  'ض': 'z',
  'ط': 't',
  'ظ': 'z',
  'ع': 'a',
  'غ': 'gh',
  'ف': 'f',
  'ق': 'gh',
  'ک': 'k',
  'ك': 'k',
  'گ': 'g',
  'ل': 'l',
  'م': 'm',
  'ن': 'n',
  'و': 'v',
  'ه': 'h',
  'ة': 'h',
  'ی': 'i',
  'ي': 'i',
  'ئ': 'y',
  'ء': "'",
  '؟': '?',
  '،': ',',
  '؛': ';',
  '٪': '%',
  'ـ': '',
  '\u200c': '',
  '\u200d': '',
  '\u200e': '',
  '\u200f': '',
  '\ufeff': '',
};

const ARABIC_DIACRITICS = /[\u064B-\u065F\u0670\u06D6-\u06ED]/;

export function transliteratePersian(input: string): string {
  let out = '';
  for (const char of input) {
    if (PERSIAN_DIGITS[char]) {
      out += PERSIAN_DIGITS[char];
      continue;
    }
    if (ARABIC_DIACRITICS.test(char)) {
      continue;
    }
    if (LETTERS[char] !== undefined) {
      out += LETTERS[char];
      continue;
    }
    const decomposed = char.normalize('NFD').replace(/[\u0300-\u036f]/g, '');
    out += decomposed;
  }
  return out.replace(/[ \t]+\n/g, '\n').replace(/ {3,}/g, '  ');
}

export function latinOnly(input: string): string {
  const transliterated = transliteratePersian(input);
  return transliterated
    .split('')
    .map((char) => {
      const code = char.charCodeAt(0);
      if (code >= 0x20 && code <= 0x7e) return char;
      if (char === '\n' || char === '\r' || char === '\t') return char;
      return '';
    })
    .join('')
    .replace(/[ \t]{3,}/g, '  ');
}

export function applyTextMode(text: string, mode: TextMode): string {
  if (mode === 'native') return text;
  if (mode === 'finglish') return transliteratePersian(text);
  return latinOnly(text);
}

export function hasPersianScript(text: string): boolean {
  return /[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF\uFB50-\uFDFF\uFE70-\uFEFF]/.test(text);
}

export function autoDirection(text: string): 'rtl' | 'ltr' {
  return hasPersianScript(text) ? 'rtl' : 'ltr';
}
