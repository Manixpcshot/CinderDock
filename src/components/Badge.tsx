import type { ReactNode } from 'react';
import { classNames } from '../lib/format';

type Tone = 'neutral' | 'accent' | 'success' | 'warning' | 'danger' | 'muted';

export function Badge({
  children,
  tone = 'neutral',
  className,
  title,
}: {
  children: ReactNode;
  tone?: Tone;
  className?: string;
  title?: string;
}) {
  return (
    <span className={classNames('cd-badge', `cd-badge--${tone}`, className)} title={title}>
      {children}
    </span>
  );
}
