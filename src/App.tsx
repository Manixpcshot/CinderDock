import { AlertTriangle, RefreshCw } from 'lucide-react';
import { useApp } from './state/AppContext';
import { classNames } from './lib/format';
import { Sidebar } from './components/Sidebar';
import { Topbar } from './components/Topbar';
import { StartScreen } from './components/StartScreen';
import { ChatView } from './components/ChatView';
import { Composer } from './components/Composer';
import { Inspector } from './components/Inspector';
import { ToastHost } from './components/Toast';
import { ApprovalDialog } from './components/ApprovalDialog';
import { ProviderDialog } from './components/ProviderDialog';
import { ModelManagerDialog } from './components/ModelManagerDialog';
import { SettingsDialog } from './components/SettingsDialog';
import { SearchDialog } from './components/SearchDialog';
import { ShortcutsDialog } from './components/ShortcutsDialog';
import { Button } from './components/Button';

function BootErrorScreen() {
  const { t, bootError, refreshBootstrap, loading } = useApp();
  return (
    <div className="cd-boot-error">
      <img src="/cinderdock.svg" alt="" />
      <h1>{t('backendUnavailable')}</h1>
      <p>{t('backendUnavailableBody')}</p>
      {bootError ? <code dir="auto">{bootError}</code> : null}
      <Button
        variant="primary"
        icon={<RefreshCw size={15} className={loading ? 'cd-spin' : ''} />}
        onClick={() => void refreshBootstrap().catch(() => undefined)}
        loading={loading}
      >
        {t('retryBackend')}
      </Button>
      <div className="cd-boot-error__hint">
        <AlertTriangle size={14} />
        {t('focusHint')}
      </div>
    </div>
  );
}

function LoadingScreen() {
  const { t } = useApp();
  return (
    <div className="cd-loading" aria-live="polite">
      <img src="/cinderdock.svg" alt="" className="cd-loading__logo" />
      <div className="cd-skeleton cd-skeleton--title" />
      <div className="cd-skeleton cd-skeleton--line" />
      <div className="cd-skeleton cd-skeleton--line short" />
      <span>{t('loading')}</span>
    </div>
  );
}

export default function App() {
  const {
    bootError,
    loading,
    sidebarOpen,
    inspectorOpen,
    activeConversationId,
    modal,
    approval,
  } = useApp();

  if (bootError) return <BootErrorScreen />;
  if (loading) return <LoadingScreen />;

  return (
    <div
      className={classNames(
        'cd-app',
        !sidebarOpen && 'cd-app--sidebar-collapsed',
        !inspectorOpen && 'cd-app--inspector-closed',
      )}
    >
      <Sidebar />
      <main className="cd-main">
        <Topbar />
        <div className="cd-main__body">
          {activeConversationId ? <ChatView /> : <StartScreen />}
          {activeConversationId ? (
            <div className="cd-main__composer">
              <Composer />
            </div>
          ) : null}
        </div>
      </main>
      {inspectorOpen ? <Inspector /> : null}
      <ToastHost />
      {approval ? <ApprovalDialog /> : null}
      {modal === 'providers' ? <ProviderDialog /> : null}
      {modal === 'models' ? <ModelManagerDialog /> : null}
      {modal === 'settings' ? <SettingsDialog /> : null}
      {modal === 'search' ? <SearchDialog /> : null}
      {modal === 'shortcuts' ? <ShortcutsDialog /> : null}
    </div>
  );
}
