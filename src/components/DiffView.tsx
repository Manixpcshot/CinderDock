import { parseUnifiedDiff } from '../lib/diff';
import { classNames } from '../lib/format';

export function DiffView({ diff, maxLines = 900 }: { diff: string; maxLines?: number }) {
  const lines = parseUnifiedDiff(diff);
  const visible = lines.slice(0, maxLines);
  return (
    <div className="cd-diff" dir="ltr" aria-label="Unified diff">
      <div className="cd-diff__lines" role="table">
        {visible.map((line, index) => (
          <div key={index} className={classNames('cd-diff__line', `cd-diff__line--${line.type}`)} role="row">
            <span className="cd-diff__number" role="cell">
              {line.oldNumber ?? ''}
            </span>
            <span className="cd-diff__number" role="cell">
              {line.newNumber ?? ''}
            </span>
            <span className="cd-diff__marker" role="cell">
              {line.type === 'add' ? '+' : line.type === 'remove' ? '-' : ''}
            </span>
            <code className="cd-diff__text" role="cell">
              {line.text || ' '}
            </code>
          </div>
        ))}
        {lines.length > maxLines ? (
          <div className="cd-diff__line cd-diff__line--meta" role="row">
            <span />
            <span />
            <span />
            <code className="cd-diff__text">
              … {lines.length - maxLines} more diff lines are not rendered.
            </code>
          </div>
        ) : null}
      </div>
    </div>
  );
}
