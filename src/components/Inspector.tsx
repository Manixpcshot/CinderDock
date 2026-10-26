import { useState } from 'react';
import {
  Activity,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  CircleDashed,
  FileCode2,
  FolderTree,
  RotateCcw,
  TerminalSquare,
  Trash2,
  XCircle,
} from 'lucide-react';
import { useApp } from '../state/AppContext';
import { classNames, relativeTime, shortPath } from '../lib/format';
import { Badge } from './Badge';
import { Button, IconButton } from './Button';
import { DiffView } from './DiffView';
import { EmptyState } from './EmptyState';

function statusTone(status: string): 'success' | 'danger' | 'warning' | 'accent' | 'muted' {
  if (['success', 'once', 'online', 'applied', 'approved'].includes(status)) return 'success';
  if (['error', 'denied', 'deny', 'offline', 'critical'].includes(status)) return 'danger';
  if (['pending', 'running', 'retrying', 'approval_required'].includes(status)) return 'warning';
  if (['session', 'auto_session'].includes(status)) return 'accent';
  return 'muted';
}

export function Inspector() {
  const {
    t,
    d,
    inspectorTab,
    setInspectorTab,
    toolEvents,
    fileChanges,
    terminalLines,
    terminalVisible,
    toggleTerminal,
    clearTerminal,
    revertChange,
    projectFiles,
    settings,
  } = useApp();
  const [expandedDiff, setExpandedDiff] = useState<Record<string, boolean>>({});
  const [expandedEvents, setExpandedEvents] = useState<Record<string, boolean>>({});

  return (
    <aside className="cd-inspector" aria-label={t('toolActivity')}>
      <div className="cd-inspector__tabs" role="tablist">
        <button
          type="button"
          role="tab"
          data-testid="tab-activity"
          aria-selected={inspectorTab === 'activity'}
          className={classNames(inspectorTab === 'activity' && 'is-active')}
          onClick={() => setInspectorTab('activity')}
        >
          <Activity size={15} />
          {t('toolActivity')}
          {toolEvents.length > 0 ? <span>{toolEvents.length}</span> : null}
        </button>
        <button
          type="button"
          role="tab"
          data-testid="tab-files"
          aria-selected={inspectorTab === 'files'}
          className={classNames(inspectorTab === 'files' && 'is-active')}
          onClick={() => setInspectorTab('files')}
        >
          <FileCode2 size={15} />
          {t('files')}
          {fileChanges.length > 0 ? <span>{fileChanges.length}</span> : null}
        </button>
        <button
          type="button"
          role="tab"
          data-testid="tab-terminal"
          aria-selected={inspectorTab === 'terminal'}
          className={classNames(inspectorTab === 'terminal' && 'is-active')}
          onClick={() => {
            setInspectorTab('terminal');
            if (!terminalVisible) toggleTerminal();
          }}
        >
          <TerminalSquare size={15} />
          {t('terminal')}
          {terminalLines.length > 0 ? <span>{terminalLines.length}</span> : null}
        </button>
      </div>

      <div className="cd-inspector__content">
        {inspectorTab === 'activity' ? (
          toolEvents.length === 0 ? (
            <EmptyState
              icon={<CircleDashed size={24} />}
              title={t('noActivity')}
              compact
            />
          ) : (
            <ul className="cd-activity-list">
              {toolEvents.map((event) => (
                <li key={event.id} className={`cd-activity-item cd-activity-item--${event.kind}`}>
                  <div className="cd-activity-item__top">
                    <span className="cd-activity-item__icon">
                      {event.status === 'success' ? (
                        <CheckCircle2 size={15} />
                      ) : event.status === 'error' || event.status === 'denied' ? (
                        <XCircle size={15} />
                      ) : (
                        <Activity size={15} />
                      )}
                    </span>
                    <div>
                      <strong dir="auto">{d(event.summary)}</strong>
                      <small>
                        <code dir="ltr">{event.name}</code> ·{' '}
                        {relativeTime(
                          event.createdAt,
                          settings?.preferences.uiLanguage === 'en' ? 'en-US' : 'fa-IR',
                        )}
                      </small>
                    </div>
                    <Badge tone={statusTone(event.status)}>{event.status}</Badge>
                  </div>
                  {event.details ? (
                    <>
                      <button
                        type="button"
                        className="cd-activity-item__toggle"
                        onClick={() =>
                          setExpandedEvents((current) => ({
                            ...current,
                            [event.id]: !current[event.id],
                          }))
                        }
                      >
                        {expandedEvents[event.id] ? <ChevronUp size={13} /> : <ChevronDown size={13} />}
                        {t('details')}
                      </button>
                      {expandedEvents[event.id] ? (
                        <pre dir="auto">{d(event.details)}</pre>
                      ) : null}
                    </>
                  ) : null}
                </li>
              ))}
            </ul>
          )
        ) : null}

        {inspectorTab === 'files' ? (
          <div className="cd-files-panel">
            {fileChanges.length === 0 ? (
              <EmptyState icon={<FileCode2 size={24} />} title={t('noChanges')} compact />
            ) : (
              <ul className="cd-change-list">
                {fileChanges.map((change) => (
                  <li key={change.id}>
                    <div className="cd-change-list__header">
                      <div>
                        <strong title={change.path} dir="ltr">{shortPath(change.path, 42)}</strong>
                        <small>{relativeTime(change.createdAt)}</small>
                      </div>
                      <Badge tone={statusTone(change.status)}>{change.status}</Badge>
                    </div>
                    <div className="cd-change-list__actions">
                      <Button
                        variant="quiet"
                        size="sm"
                        icon={expandedDiff[change.id] ? <ChevronUp size={13} /> : <ChevronDown size={13} />}
                        onClick={() =>
                          setExpandedDiff((current) => ({
                            ...current,
                            [change.id]: !current[change.id],
                          }))
                        }
                      >
                        {expandedDiff[change.id] ? t('hideDiff') : t('showDiff')}
                      </Button>
                      {change.status === 'applied' ? (
                        <Button
                          variant="quiet"
                          size="sm"
                          icon={<RotateCcw size={13} />}
                          onClick={() => void revertChange(change.id)}
                        >
                          {t('revert')}
                        </Button>
                      ) : null}
                    </div>
                    {expandedDiff[change.id] ? <DiffView diff={change.diff} /> : null}
                  </li>
                ))}
              </ul>
            )}
            <section className="cd-project-files">
              <h3>
                <FolderTree size={15} />
                {t('currentProject')}
              </h3>
              {projectFiles.length === 0 ? (
                <p>{t('noProjects')}</p>
              ) : (
                <ul>
                  {projectFiles.slice(0, 120).map((file) => (
                    <li key={file} title={file} dir="ltr">
                      {shortPath(file, 44)}
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </div>
        ) : null}

        {inspectorTab === 'terminal' ? (
          <div className="cd-terminal-panel">
            <div className="cd-terminal-panel__toolbar">
              <span>{t('commandOutput')}</span>
              <div>
                <IconButton label={t('clearTerminal')} size="sm" onClick={clearTerminal}>
                  <Trash2 size={14} />
                </IconButton>
                <IconButton label={t('collapseTerminal')} size="sm" onClick={toggleTerminal}>
                  <ChevronDown size={14} />
                </IconButton>
              </div>
            </div>
            <div className="cd-terminal-panel__body" aria-live="polite">
              {terminalLines.length === 0 ? (
                <EmptyState icon={<TerminalSquare size={24} />} title={t('noTerminal')} compact />
              ) : (
                terminalLines.map((line, index) => (
                  <div key={`${index}-${line.line}`} className={`cd-terminal-line cd-terminal-line--${line.stream}`}>
                    <span>{line.stream === 'stderr' ? '!' : '›'}</span>
                    <code>{d(line.line)}</code>
                  </div>
                ))
              )}
            </div>
          </div>
        ) : null}
      </div>

      {terminalVisible && inspectorTab !== 'terminal' ? (
        <div className="cd-terminal-drawer">
          <div className="cd-terminal-drawer__header">
            <strong>{t('terminal')}</strong>
            <div>
              <IconButton label={t('clearTerminal')} size="sm" onClick={clearTerminal}>
                <Trash2 size={13} />
              </IconButton>
              <IconButton label={t('collapseTerminal')} size="sm" onClick={toggleTerminal}>
                <ChevronDown size={13} />
              </IconButton>
            </div>
          </div>
          <div className="cd-terminal-drawer__body">
            {terminalLines.length === 0 ? (
              <span>{t('noTerminal')}</span>
            ) : (
              terminalLines.slice(-80).map((line, index) => (
                <div key={`${index}-drawer`} className={`cd-terminal-line cd-terminal-line--${line.stream}`}>
                  <span>{line.stream === 'stderr' ? '!' : '›'}</span>
                  <code>{d(line.line)}</code>
                </div>
              ))
            )}
          </div>
        </div>
      ) : null}
    </aside>
  );
}
