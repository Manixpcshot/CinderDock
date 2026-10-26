import { useEffect, useRef } from 'react';
import { ArrowUp, FolderOpen, Paperclip, Square, X } from 'lucide-react';
import { useApp } from '../state/AppContext';
import { classNames, titleFromPath } from '../lib/format';
import { Button, IconButton } from './Button';
import { ModelSelector } from './ModelSelector';

export function Composer({ variant = 'inline' }: { variant?: 'hero' | 'inline' }) {
  const {
    t,
    d,
    composer,
    setComposer,
    attachments,
    removeAttachment,
    chooseAttachments,
    chooseProject,
    settings,
    sendMessage,
    stopGeneration,
    streaming,
    runId,
    visionWarning,
    openModal,
  } = useApp();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const node = textareaRef.current;
    if (!node) return;
    node.style.height = 'auto';
    node.style.height = `${Math.min(node.scrollHeight, variant === 'hero' ? 210 : 160)}px`;
  }, [composer, variant]);

  useEffect(() => {
    if (variant === 'hero') textareaRef.current?.focus();
  }, [variant]);

  return (
    <div className={classNames('cd-composer', `cd-composer--${variant}`)}>
      {attachments.length > 0 ? (
        <div className="cd-composer__attachments" aria-label={t('attach')}>
          {attachments.map((attachment) => (
            <span key={attachment.path} className="cd-attachment-chip" title={attachment.path}>
              <Paperclip size={13} />
              <span dir="ltr">{d(titleFromPath(attachment.path))}</span>
              <button
                type="button"
                onClick={() => removeAttachment(attachment.path)}
                aria-label={`${t('delete')} ${attachment.path}`}
              >
                <X size={12} />
              </button>
            </span>
          ))}
        </div>
      ) : null}
      {visionWarning ? (
        <div className="cd-composer__warning" role="status">
          {t('visionWarning')}
        </div>
      ) : null}
      <textarea
        ref={textareaRef}
        className="cd-composer__input"
        value={composer}
        onChange={(event) => setComposer(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            if (!streaming) void sendMessage();
          }
        }}
        placeholder={t('composerPlaceholder')}
        aria-label={t('composerPlaceholder')}
        rows={variant === 'hero' ? 3 : 1}
        dir="auto"
      />
      <div className="cd-composer__toolbar">
        <div className="cd-composer__toolbar-left">
          <IconButton label={t('attach')} onClick={chooseAttachments} size="sm">
            <Paperclip size={16} />
          </IconButton>
          <button type="button" className="cd-composer__project" onClick={chooseProject} title={settings?.projectPath ?? t('chooseProject')}>
            <FolderOpen size={14} />
            <span dir="ltr">
              {settings?.projectPath ? d(titleFromPath(settings.projectPath)) : t('projectOrFolder')}
            </span>
          </button>
          <ModelSelector compact />
        </div>
        <div className="cd-composer__toolbar-right">
          <span className="cd-composer__hint">
            <kbd>↵</kbd> {t('send')} · <kbd>⇧↵</kbd> {t('shortcutNewline')}
          </span>
          {streaming ? (
            <Button
              variant="danger"
              size="sm"
              icon={<Square size={13} />}
              onClick={() => void stopGeneration()}
              disabled={!runId}
            >
              {t('stop')}
            </Button>
          ) : (
            <IconButton
              label={t('send')}
              variant="primary"
              size="md"
              onClick={() => void sendMessage()}
              disabled={!composer.trim() && attachments.length === 0}
              className="cd-composer__send"
            >
              <ArrowUp size={17} />
            </IconButton>
          )}
        </div>
      </div>
      {!settings?.models.length ? (
        <button
          type="button"
          className="cd-composer__inline-warning"
          onClick={() => openModal('providers')}
        >
          {t('addProviderFirst')}
        </button>
      ) : null}
    </div>
  );
}
