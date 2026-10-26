import { Keyboard } from 'lucide-react';
import { useApp } from '../state/AppContext';
import { Button } from './Button';
import { Modal } from './Modal';

export function ShortcutsDialog() {
  const { t, closeModal } = useApp();
  const items: Array<[string, string]> = [
    ['Ctrl / ⌘ + N', t('shortcutNew')],
    ['Ctrl / ⌘ + K', t('shortcutSearch')],
    ['Ctrl / ⌘ + ,', t('shortcutSettings')],
    ['Ctrl / ⌘ + B', t('shortcutSidebar')],
    ['Ctrl / ⌘ + Shift + T', t('shortcutTerminal')],
    ['Alt + 1 / 2 / 3', t('shortcutTabs')],
    ['Enter', t('shortcutSend')],
    ['Shift + Enter', t('shortcutNewline')],
    ['Esc', t('shortcutClose')],
  ];
  return (
    <Modal
      title={t('shortcuts')}
      onClose={closeModal}
      size="sm"
      footer={
        <>
          <div className="cd-modal__footer-spacer" />
          <Button variant="primary" onClick={closeModal}>
            {t('close')}
          </Button>
        </>
      }
    >
      <div className="cd-shortcuts">
        <Keyboard size={22} />
        <ul>
          {items.map(([keys, label]) => (
            <li key={keys}>
              <kbd dir="ltr">{keys}</kbd>
              <span>{label}</span>
            </li>
          ))}
        </ul>
      </div>
    </Modal>
  );
}
