import { useEffect, useState } from 'react';
import { KeyRound, Monitor, RefreshCw, ShieldCheck, SlidersHorizontal } from 'lucide-react';
import { useApp } from '../state/AppContext';
import type { Preferences, TextMode } from '../lib/types';
import { Badge } from './Badge';
import { Button } from './Button';
import { Modal } from './Modal';

export function SettingsDialog() {
  const {
    t,
    d,
    closeModal,
    preferences,
    updatePreferences,
    clearSessionGrants,
    settings,
    providerBusy,
  } = useApp();
  const [draft, setDraft] = useState<Preferences>(preferences);

  useEffect(() => setDraft(preferences), [preferences]);

  function patch(update: Partial<Preferences>) {
    setDraft((current) => ({ ...current, ...update }));
  }

  return (
    <Modal
      title={t('settings')}
      subtitle={t('security')}
      onClose={closeModal}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={closeModal}>
            {t('cancel')}
          </Button>
          <div className="cd-modal__footer-spacer" />
          <Button
            variant="primary"
            onClick={() => void updatePreferences(draft)}
            loading={providerBusy}
          >
            {t('save')}
          </Button>
        </>
      }
    >
      <div className="cd-settings">
        <section className="cd-settings__section">
          <h3><SlidersHorizontal size={16} /> {t('textMode')}</h3>
          <div className="cd-segmented">
            {(['native', 'finglish', 'latin'] as TextMode[]).map((mode) => (
              <button
                key={mode}
                type="button"
                className={draft.textMode === mode ? 'is-active' : ''}
                onClick={() => patch({ textMode: mode })}
              >
                {t(mode)}
              </button>
            ))}
          </div>
          <p className="cd-settings__hint">{t('focusHint')}</p>
        </section>

        <section className="cd-settings__section">
          <h3><Monitor size={16} /> {t('interfaceLanguage')}</h3>
          <div className="cd-form-grid">
            <label className="cd-field">
              <span>{t('interfaceLanguage')}</span>
              <select
                value={draft.uiLanguage}
                onChange={(event) => patch({ uiLanguage: event.target.value as 'fa' | 'en' })}
              >
                <option value="fa">فارسی</option>
                <option value="en">English</option>
              </select>
            </label>
            <label className="cd-field">
              <span>{t('fontScale')}: {draft.fontScale.toFixed(2)}×</span>
              <input
                type="range"
                min={0.85}
                max={1.35}
                step={0.05}
                value={draft.fontScale}
                onChange={(event) => patch({ fontScale: Number(event.target.value) })}
              />
            </label>
          </div>
        </section>

        <section className="cd-settings__section">
          <h3><ShieldCheck size={16} /> {t('security')}</h3>
          <div className="cd-settings__switches">
            <label>
              <input
                type="checkbox"
                checked={draft.desktopToolsEnabled}
                onChange={(event) => patch({ desktopToolsEnabled: event.target.checked })}
              />
              <span>
                <strong>{t('desktopTools')}</strong>
                <small>{t('desktopToolsHint')}</small>
              </span>
            </label>
            <label>
              <input
                type="checkbox"
                checked={draft.webAccessEnabled}
                onChange={(event) => patch({ webAccessEnabled: event.target.checked })}
              />
              <span>
                <strong>{t('webAccess')}</strong>
                <small>{t('apiKeyWarning')}</small>
              </span>
            </label>
            <label>
              <input
                type="checkbox"
                checked={draft.terminalVisible}
                onChange={(event) => patch({ terminalVisible: event.target.checked })}
              />
              <span>
                <strong>{t('terminal')}</strong>
                <small>{t('commandOutput')}</small>
              </span>
            </label>
          </div>
          <div className="cd-form-grid cd-settings__numbers">
            <label className="cd-field">
              <span>{t('maxIterations')}</span>
              <input
                type="number"
                min={1}
                max={16}
                value={draft.maxToolIterations}
                onChange={(event) => patch({ maxToolIterations: Number(event.target.value) })}
              />
            </label>
            <label className="cd-field">
              <span>{t('retries')}</span>
              <input
                type="number"
                min={0}
                max={5}
                value={draft.maxRetries}
                onChange={(event) => patch({ maxRetries: Number(event.target.value) })}
              />
            </label>
          </div>
          <Button
            variant="secondary"
            size="sm"
            icon={<RefreshCw size={13} />}
            onClick={() => void clearSessionGrants()}
          >
            {t('clearGrants')}
          </Button>
        </section>

        <section className="cd-settings__section">
          <h3><KeyRound size={16} /> {t('dataLocation')}</h3>
          <dl className="cd-settings__facts">
            <div>
              <dt>{t('version')}</dt>
              <dd dir="ltr">{settings?.app.version ?? '0.1.0'}</dd>
            </div>
            <div>
              <dt>{t('platform')}</dt>
              <dd dir="ltr">{settings?.app.platform ?? 'windows'}</dd>
            </div>
            <div>
              <dt>{t('secretBackend')}</dt>
              <dd dir="auto">{d(settings?.app.secretBackend ?? '')}</dd>
            </div>
            <div>
              <dt>{t('dataLocation')}</dt>
              <dd dir="ltr" title={settings?.app.dataDir}>{settings?.app.dataDir ?? '—'}</dd>
            </div>
          </dl>
          <div className="cd-settings__updates">
            <Badge tone="muted">{t('updates')}: {t('offline')}</Badge>
            <p>{t('updateDisabledReason')}</p>
          </div>
        </section>
      </div>
    </Modal>
  );
}
