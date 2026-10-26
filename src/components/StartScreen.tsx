import { ArrowRight, FolderOpen, PlugZap, Settings2, Sparkles } from 'lucide-react';
import { useApp } from '../state/AppContext';
import { Button } from './Button';
import { Composer } from './Composer';
import { Badge } from './Badge';

export function StartScreen() {
  const { t, d, settings, setComposer, openModal, startProviderWizard, chooseProject, connection } = useApp();
  const suggestions = [t('suggestion1'), t('suggestion2'), t('suggestion3')];

  return (
    <div className="cd-start">
      <div className="cd-start__glow" aria-hidden="true" />
      <div className="cd-start__inner">
        <div className="cd-start__mark">
          <img src="/cinderdock.svg" alt="" />
        </div>
        <h1>{t('welcome')}</h1>
        <p className="cd-start__subtitle">{t('welcomeSubtitle')}</p>
        {connection.state === 'offline' || connection.state === 'error' ? (
          <div className="cd-start__disconnected" role="status">
            <Badge tone="danger">{t('offline')}</Badge>
            <span dir="auto">{d(connection.message || t('disconnected'))}</span>
          </div>
        ) : null}
        {!settings?.models.length ? (
          <div className="cd-start__onboarding">
            <div>
              <strong>{t('addProviderFirst')}</strong>
              <span>{t('addProviderHint')}</span>
            </div>
            <Button
              variant="primary"
              icon={<ArrowRight size={15} />}
              onClick={() => startProviderWizard()}
            >
              {t('addProvider')}
            </Button>
          </div>
        ) : null}
        <Composer variant="hero" />
        <div className="cd-start__suggestions">
          {suggestions.map((suggestion) => (
            <button key={suggestion} type="button" onClick={() => setComposer(suggestion)}>
              <Sparkles size={14} />
              <span dir="auto">{d(suggestion)}</span>
            </button>
          ))}
        </div>
        <div className="cd-start__quick-actions">
          <button type="button" data-testid="quick-project" onClick={chooseProject}>
            <FolderOpen size={16} />
            <span>{t('chooseProject')}</span>
          </button>
          <button type="button" data-testid="quick-providers" onClick={() => openModal('providers')}>
            <PlugZap size={16} />
            <span>{t('providers')}</span>
          </button>
          <button type="button" data-testid="quick-settings" onClick={() => openModal('settings')}>
            <Settings2 size={16} />
            <span>{t('settings')}</span>
          </button>
        </div>
      </div>
    </div>
  );
}
