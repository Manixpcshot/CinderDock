import {
  BookOpen,
  ChevronLeft,
  ChevronRight,
  Command,
  FolderGit2,
  MessageSquarePlus,
  PlugZap,
  Search,
  Settings2,
  Trash2,
} from 'lucide-react';
import { useApp } from '../state/AppContext';
import { classNames, relativeTime, shortPath, titleFromPath } from '../lib/format';
import { Badge } from './Badge';
import { Button, IconButton } from './Button';

export function Sidebar() {
  const {
    t,
    d,
    dir,
    sidebarOpen,
    toggleSidebar,
    newChat,
    openModal,
    openConversation,
    deleteConversation,
    conversations,
    activeConversationId,
    recentProjects,
    settings,
    setProjectByPath,
    chooseProject,
    connection,
    currentProvider,
  } = useApp();

  const connectionTone =
    connection.state === 'online'
      ? 'success'
      : connection.state === 'offline' || connection.state === 'error'
        ? 'danger'
        : connection.state === 'retrying'
          ? 'warning'
          : 'muted';

  return (
    <aside className={classNames('cd-sidebar', !sidebarOpen && 'is-collapsed')} aria-label={t('projects')}>
      <div className="cd-sidebar__brand">
        <img src="/cinderdock.svg" alt="" className="cd-sidebar__logo" />
        {sidebarOpen ? (
          <div className="cd-sidebar__brand-text">
            <strong>CinderDock</strong>
            <span>{t('tagline')}</span>
          </div>
        ) : null}
        <IconButton
          label={sidebarOpen ? t('collapseTerminal') : t('expandTerminal')}
          onClick={toggleSidebar}
          size="sm"
          className="cd-sidebar__collapse"
        >
          {sidebarOpen ? (
            dir === 'rtl' ? <ChevronRight size={16} /> : <ChevronLeft size={16} />
          ) : dir === 'rtl' ? (
            <ChevronLeft size={16} />
          ) : (
            <ChevronRight size={16} />
          )}
        </IconButton>
      </div>

      <div className="cd-sidebar__new">
        <Button
          variant="secondary"
          size="md"
          icon={<MessageSquarePlus size={17} />}
          onClick={newChat}
          title={t('newChat')}
        >
          {sidebarOpen ? t('newChat') : null}
        </Button>
      </div>

      <nav className="cd-sidebar__nav" aria-label="Main navigation">
        <button type="button" data-testid="nav-search" onClick={() => openModal('search')} title={t('search')}>
          <Search size={18} />
          {sidebarOpen ? <span>{t('search')}</span> : null}
          {sidebarOpen ? <kbd>⌘K</kbd> : null}
        </button>
        <button type="button" data-testid="nav-project" onClick={chooseProject} title={t('chooseProject')}>
          <FolderGit2 size={18} />
          {sidebarOpen ? <span>{t('chooseProject')}</span> : null}
        </button>
        <button type="button" data-testid="nav-providers" onClick={() => openModal('providers')} title={t('providers')}>
          <PlugZap size={18} />
          {sidebarOpen ? <span>{t('providers')}</span> : null}
        </button>
        <button type="button" data-testid="nav-models" onClick={() => openModal('models')} title={t('manageModels')}>
          <Command size={18} />
          {sidebarOpen ? <span>{t('manageModels')}</span> : null}
        </button>
        <button type="button" data-testid="nav-shortcuts" onClick={() => openModal('shortcuts')} title={t('shortcuts')}>
          <BookOpen size={18} />
          {sidebarOpen ? <span>{t('shortcuts')}</span> : null}
        </button>
      </nav>

      {sidebarOpen ? (
        <div className="cd-sidebar__scroll">
          <section className="cd-sidebar__section">
            <h2>{t('recent')}</h2>
            {conversations.length === 0 ? (
              <p className="cd-sidebar__muted">{t('noConversations')}</p>
            ) : (
              <ul className="cd-conversation-list">
                {conversations.slice(0, 12).map((conversation) => (
                  <li key={conversation.id}>
                    <button
                      type="button"
                      className={classNames(
                        'cd-conversation-item',
                        activeConversationId === conversation.id && 'is-active',
                      )}
                      onClick={() => openConversation(conversation.id)}
                    >
                      <span className="cd-conversation-item__title" dir="auto">
                        {d(conversation.title)}
                      </span>
                      <span className="cd-conversation-item__meta">
                        {relativeTime(conversation.updatedAt, settings?.preferences.uiLanguage === 'en' ? 'en-US' : 'fa-IR')}
                      </span>
                    </button>
                    <IconButton
                      label={t('deleteConversation')}
                      size="sm"
                      onClick={() => deleteConversation(conversation.id)}
                      className="cd-conversation-item__delete"
                    >
                      <Trash2 size={14} />
                    </IconButton>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className="cd-sidebar__section">
            <h2>{t('projects')}</h2>
            {recentProjects.length === 0 ? (
              <p className="cd-sidebar__muted">{t('noProjects')}</p>
            ) : (
              <ul className="cd-project-list">
                {recentProjects.map((project) => (
                  <li key={project}>
                    <button type="button" onClick={() => setProjectByPath(project)} title={project}>
                      <FolderGit2 size={15} />
                      <span dir="ltr">{titleFromPath(project)}</span>
                    </button>
                    <small title={project}>{shortPath(project, 30)}</small>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      ) : null}

      <div className="cd-sidebar__footer">
        <div className="cd-sidebar__status" title={connection.message || currentProvider?.displayName || ''}>
          <span className={`cd-status-dot cd-status-dot--${connectionTone}`} />
          {sidebarOpen ? (
            <div>
              <strong>{connection.state === 'online' ? t('online') : connection.state === 'unknown' ? t('unknown') : connection.state === 'offline' ? t('offline') : t('status')}</strong>
              <small>{currentProvider ? d(currentProvider.displayName) : t('noProviders')}</small>
            </div>
          ) : null}
        </div>
        <button type="button" data-testid="nav-settings" onClick={() => openModal('settings')} title={t('settings')}>
          <Settings2 size={18} />
          {sidebarOpen ? <span>{t('settings')}</span> : null}
        </button>
        {sidebarOpen ? (
          <Badge tone="muted" className="cd-sidebar__version">
            v{settings?.app.version ?? '0.1.0'}
          </Badge>
        ) : null}
      </div>
    </aside>
  );
}
