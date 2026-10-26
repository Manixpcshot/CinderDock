export type DiffLineType = 'meta' | 'hunk' | 'add' | 'remove' | 'context';

export interface DiffLine {
  type: DiffLineType;
  text: string;
  oldNumber?: number;
  newNumber?: number;
}

export function parseUnifiedDiff(diff: string): DiffLine[] {
  const lines = diff.split('\n');
  let oldLine = 0;
  let newLine = 0;
  return lines.map((text) => {
    if (text.startsWith('---') || text.startsWith('+++')) {
      return { type: 'meta' as const, text };
    }
    if (text.startsWith('@@')) {
      const match = /@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(text);
      if (match) {
        oldLine = Number(match[1]);
        newLine = Number(match[2]);
      }
      return { type: 'hunk' as const, text, oldNumber: oldLine, newNumber: newLine };
    }
    if (text.startsWith('+')) {
      return { type: 'add' as const, text: text.slice(1), newNumber: newLine++ };
    }
    if (text.startsWith('-')) {
      return { type: 'remove' as const, text: text.slice(1), oldNumber: oldLine++ };
    }
    return { type: 'context' as const, text: text.startsWith(' ') ? text.slice(1) : text, oldNumber: oldLine++, newNumber: newLine++ };
  });
}
