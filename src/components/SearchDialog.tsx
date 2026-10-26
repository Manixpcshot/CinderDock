import { MessageSquareText, Search, Trash2 } from 'lucide-react';
import { useApp } from '../state/AppContext';
import { relativeTime } from '../lib/format';
import { EmptyState } from './EmptyState';
import { IconButton } from './Button';
import { Modal } from './Modal';

export function SearchDialog() {
  const {
    t,
    d,
    closeModal,
    searchQuery,
    searchResults,
    conversations,
    searchConversations,
    openConversation,
    deleteConversation,
    settings,
  } = useApp();
  const results = searchQuery.trim()
    ? searchResults
    : conversations;

  return (
    <Modal title={t('search')} subtitle={t('searchConversations')} onClose={closeModal} size="md">
      <div className="cd-search-dialog">
        <div className="cd-search-field cd-search-field--large">
          <Search size={17} />
          <input
            autoFocus
            value={searchQuery}
            onChange={(event) => void searchConversations(event.target.value)}
            placeholder={t('searchConversations')}
            aria-label={t('searchConversations')}
          />
        </div>
        {results.length === 0 ? (
          <EmptyState icon={<MessageSquareText size={24} />} title={t('noConversations')} compact />
        ) : (
          <ul className="cd-search-results">
            {results.map((conversation) => (
              <li key={conversation.id}>
                <button type="button" onClick={() => openConversation(conversation.id)}>
                  <strong dir="auto">{d(conversation.title)}</strong>
                  <span dir="auto">{d(conversation.preview)}</span>
                  <small>
                    {relativeTime(
                      conversation.updatedAt,
                      settings?.preferences.uiLanguage === 'en' ? 'en-US' : 'fa-IR',
                    )}{' '}
                    · {conversation.messageCount}
                  </small>
                </button>
                <IconButton
                  label={t('deleteConversation')}
                  size="sm"
                  onClick={() => void deleteConversation(conversation.id)}
                >
                  <Trash2 size={14} />
                </IconButton>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Modal>
  );
}
