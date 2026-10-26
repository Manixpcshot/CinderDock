import type { ReactNode } from 'react';

export function EmptyState({
  icon,
  title,
  description,
  action,
  compact = false,
}: {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
  compact?: boolean;
}) {
  return (
    <div className={compact ? 'cd-empty cd-empty--compact' : 'cd-empty'}>
      <div className="cd-empty__icon">{icon}</div>
      <h3>{title}</h3>
      {description ? <p>{description}</p> : null}
      {action ? <div className="cd-empty__action">{action}</div> : null}
    </div>
  );
}
