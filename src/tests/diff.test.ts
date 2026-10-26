import { describe, expect, it } from 'vitest';
import { parseUnifiedDiff } from '../lib/diff';

describe('unified diff parser', () => {
  it('classifies headers, additions and removals', () => {
    const diff = [
      '--- a/file.txt',
      '+++ b/file.txt',
      '@@ -1,2 +1,2 @@',
      ' context',
      '-old',
      '+new',
    ].join('\n');
    const lines = parseUnifiedDiff(diff);
    expect(lines[0].type).toBe('meta');
    expect(lines[2].type).toBe('hunk');
    expect(lines[4]).toMatchObject({ type: 'remove', text: 'old' });
    expect(lines[5]).toMatchObject({ type: 'add', text: 'new' });
  });

  it('tracks line numbers inside a hunk', () => {
    const diff = ['@@ -10,1 +20,1 @@', ' keep', '-a', '+b'].join('\n');
    const lines = parseUnifiedDiff(diff);
    expect(lines[1].oldNumber).toBe(10);
    expect(lines[1].newNumber).toBe(20);
    expect(lines[2].oldNumber).toBe(11);
    expect(lines[3].newNumber).toBe(21);
  });
});
