import { useEffect, useRef, useState } from 'react';
import {
  Bot,
  Check,
  Copy,
  FileText,
  LoaderCircle,
  RefreshCw,
  User,
  Wrench,
} from 'lucide-react';
import { useApp } from '../state/AppContext';
import { classNames, formatBytes, relativeTime } from '../lib/format';
import { Badge } from './Badge';
import { Button, IconButton } from './Button';
import { EmptyState } from './EmptyState';
import { Markdown } from './Markdown';

export function ChatView() {
  const {
    t,
    d,
    messages,
    streaming,
    streamingContent,
    streamingMessageId,
    status,
    statusMessage,
    retryLast,
    showToast,
    settings,
    activeConversationId,
  } = useApp();
  const bottomRef = useRef<HTMLDivElement>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [expandedTools, setExpandedTools] = useState<Record<string, boolean>>({});
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: streaming ? 'smooth' : 'auto', block: 'end' });
  }, [messages.length, streamingContent, streaming]);

  async function copyMessage(id: string, content: string) {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedId(id);
      window.setTimeout(() => setCopiedId(null), 1600);
    } catch {
      showToast(t('copy') + ' —', 'error');
    }
  }

  const visibleMessages = messages.filter((message) => message.role !== 'tool');
  const lastAssistant = [...messages].reverse().find((message) => message.role === 'assistant');

  return (
    <section className="cd-chat" aria-label={t('conversationTitle')}>
      <div className="cd-chat__scroll" ref={scrollRef}>
        <div className="cd-chat__thread">
          {visibleMessages.length === 0 && !streaming ? (
            <EmptyState
              icon={<Bot size={26} />}
              title={t('emptyStateTitle')}
              description={t('emptyStateBody')}
            />
          ) : null}
          {visibleMessages.map((message) => {
            const isUser = message.role === 'user';
            return (
              <article
                key={message.id}
                className={classNames('cd-message', isUser ? 'cd-message--user' : 'cd-message--assistant')}
              >
                <div className="cd-message__avatar" aria-hidden="true">
                  {isUser ? <User size={16} /> : <Bot size={16} />}
                </div>
                <div className="cd-message__body">
                  <header className="cd-message__header">
                    <strong>{isUser ? t('user') : t('assistant')}</strong>
                    <time dateTime={message.createdAt}>
                      {relativeTime(
                        message.createdAt,
                        settings?.preferences.uiLanguage === 'en' ? 'en-US' : 'fa-IR',
                      )}
                    </time>
                    {message.status === 'stopped' ? <Badge tone="warning">{t('stop')}</Badge> : null}
                    {message.status === 'error' ? <Badge tone="danger">{t('error')}</Badge> : null}
                  </header>
                  {message.attachments.length > 0 ? (
                    <div className="cd-message__attachments">
                      {message.attachments.map((attachment) => (
                        <Badge key={`${message.id}-${attachment.path}`} tone="muted" title={attachment.path}>
                          {attachment.kind === 'image' ? '🖼' : <FileText size={12} />}
                          <span dir="ltr">{d(attachment.name)}</span>
                          <small>{formatBytes(attachment.size)}</small>
                        </Badge>
                      ))}
                    </div>
                  ) : null}
                  {message.toolCalls.length > 0 ? (
                    <div className="cd-message__tool-calls">
                      {message.toolCalls.map((call) => (
                        <button
                          key={call.id}
                          type="button"
                          onClick={() =>
                            setExpandedTools((current) => ({
                              ...current,
                              [call.id]: !current[call.id],
                            }))
                          }
                        >
                          <Wrench size={13} />
                          <code>{call.name}</code>
                        </button>
                      ))}
                      {message.toolCalls.some((call) => expandedTools[call.id]) ? (
                        <pre dir="ltr">
                          {message.toolCalls
                            .filter((call) => expandedTools[call.id])
                            .map((call) => call.arguments)
                            .join('\n')}
                        </pre>
                      ) : null}
                    </div>
                  ) : null}
                  {isUser ? (
                    <div className="cd-message__text cd-message__text--user" dir="auto">
                      {d(message.content)}
                    </div>
                  ) : (
                    <Markdown content={message.content} />
                  )}
                  {!isUser ? (
                    <div className="cd-message__actions">
                      <IconButton
                        label={copiedId === message.id ? t('copied') : t('copy')}
                        size="sm"
                        onClick={() => void copyMessage(message.id, message.content)}
                      >
                        {copiedId === message.id ? <Check size={14} /> : <Copy size={14} />}
                      </IconButton>
                      {lastAssistant?.id === message.id && !streaming ? (
                        <Button
                          variant="quiet"
                          size="sm"
                          icon={<RefreshCw size={13} />}
                          onClick={() => void retryLast()}
                          title={t('retry')}
                        >
                          {t('retry')}
                        </Button>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              </article>
            );
          })}
          {streaming ? (
            <article className="cd-message cd-message--assistant is-streaming" aria-live="polite">
              <div className="cd-message__avatar" aria-hidden="true">
                <Bot size={16} />
              </div>
              <div className="cd-message__body">
                <header className="cd-message__header">
                  <strong>{t('assistant')}</strong>
                  <Badge tone="accent">
                    <LoaderCircle size={12} className="cd-spin" />
                    {status === 'retrying'
                      ? t('retrying')
                      : status === 'connecting'
                        ? t('connecting')
                        : t('streaming')}
                  </Badge>
                </header>
                {streamingContent ? (
                  <Markdown content={streamingContent} />
                ) : (
                  <div className="cd-streaming-dots" aria-label={statusMessage || t('streaming')}>
                    <span />
                    <span />
                    <span />
                  </div>
                )}
              </div>
            </article>
          ) : null}
          <div ref={bottomRef} />
        </div>
      </div>
      {activeConversationId ? (
        <div className="cd-chat__footer-note">
          <span>{t('focusHint')}</span>
          <span>{streamingMessageId ? t('streaming') : t('status')}: {status}</span>
        </div>
      ) : null}
    </section>
  );
}
