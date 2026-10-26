import { FolderOpen, PanelRightClose, PanelRightOpen, TerminalSquare, WifiOff } from 'lucide-react';
import { useApp } from '../state/AppContext';
import { classNames, titleFromPath } from '../lib/format';
import { Badge } from './Badge';
import { IconButton } from './Button';
import { ModelSelector } from './ModelSelector';

export function Topbar() {
  const {
    t,
    d,
    settings,
    projectFiles,
    chooseProject,
    inspectorOpen,
    toggleInspector,
    terminalVisible,
    toggleTerminal,
    connection,
    errorBanner,
    dismissError,
  } = useApp();

  const disconnected = connection.state === 'offline' || connection.state === 'error';

  return (
    <header className="cd-topbar">
      <div className="cd-topbar__left">
        <button type="button" className="cd-project-chip" onClick={chooseProject} title={settings?.projectPath ?? t('chooseProject')}>
          <FolderOpen size={16} />
          <span dir="ltr">
            {settings?.projectPath ? d(titleFromPath(settings.projectPath)) : t('chooseProject')}
          </span>
        </button>
        {settings?.projectPath ? (
          <Badge tone="muted" className="cd-topbar__filecount" title={t('files')}>
            {projectFiles.length}+
          </Badge>
        ) : null}
      </div>
      <div className="cd-topbar__center">
        <ModelSelector />
      </div>
      <div className="cd-topbar__right">
        {disconnected ? (
          <Badge tone="danger" title={connection.message}>
            <WifiOff size={13} />
            {t('offline')}
          </Badge>
        ) : null}
        <IconButton
          label={terminalVisible ? t('collapseTerminal') : t('expandTerminal')}
          onClick={toggleTerminal}
          active={terminalVisible}
        >
          <TerminalSquare size={17} />
        </IconButton>
        <IconButton
          label={inspectorOpen ? t('close') : t('toolActivity')}
          onClick={toggleInspector}
          active={inspectorOpen}
        >
          {inspectorOpen ? <PanelRightClose size={17} /> : <PanelRightOpen size={17} />}
        </IconButton>
      </div>
      {errorBanner ? (
        <div className={classNames('cd-error-banner')} role="alert">
          <span dir="auto">{d(errorBanner)}</span>
          <button type="button" onClick={dismissError} aria-label={t('dismiss')}>
            ×
          </button>
        </div>
      ) : null}
    </header>
  );
}
