import { useMemo, useState } from 'react';
import { Eye, Pencil, Plus, Search, Star, Trash2, Wrench } from 'lucide-react';
import { useApp } from '../state/AppContext';
import type { ModelInput, ModelView } from '../lib/types';
import { Badge } from './Badge';
import { Button, IconButton } from './Button';
import { EmptyState } from './EmptyState';
import { Modal } from './Modal';

function inputFromModel(model: ModelView): ModelInput {
  return {
    id: model.id,
    providerId: model.providerId,
    modelId: model.modelId,
    displayName: model.displayName,
    supportsTools: model.supportsTools,
    supportsVision: model.supportsVision,
    contextWindow: model.contextWindow ?? null,
    setDefault: model.isDefault,
  };
}

export function ModelManagerDialog() {
  const {
    t,
    d,
    settings,
    closeModal,
    saveModels,
    deleteModel,
    setDefaultModel,
    startProviderWizard,
    providerBusy,
  } = useApp();
  const [query, setQuery] = useState('');
  const [editing, setEditing] = useState<ModelInput | null>(null);

  const models = useMemo(() => settings?.models ?? [], [settings?.models]);
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return models;
    return models.filter((model) =>
      `${model.displayName} ${model.modelId} ${model.providerName}`
        .toLowerCase()
        .includes(needle),
    );
  }, [models, query]);

  function startNew() {
    setEditing({
      id: null,
      providerId: settings?.providers[0]?.id ?? '',
      modelId: '',
      displayName: '',
      supportsTools: false,
      supportsVision: false,
      contextWindow: null,
      setDefault: models.length === 0,
    });
  }

  async function saveEditing() {
    if (!editing || !editing.providerId || !editing.modelId.trim()) return;
    const result = await saveModels([editing]);
    if (result) setEditing(null);
  }

  return (
    <Modal
      title={t('manageModels')}
      subtitle={t('modelList')}
      onClose={closeModal}
      size="xl"
      footer={
        <>
          <Button variant="ghost" onClick={closeModal}>
            {t('close')}
          </Button>
          <div className="cd-modal__footer-spacer" />
          <Button variant="primary" icon={<Plus size={14} />} onClick={startNew} disabled={!settings?.providers.length}>
            {t('addModel')}
          </Button>
        </>
      }
    >
      <div className="cd-model-manager">
        <div className="cd-model-manager__list">
          <div className="cd-search-field">
            <Search size={15} />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t('searchModels')}
              aria-label={t('searchModels')}
            />
          </div>
          {filtered.length === 0 ? (
            <EmptyState
              icon={<Wrench size={23} />}
              title={t('noModels')}
              description={settings?.providers.length ? undefined : t('addProviderFirst')}
              compact
              action={
                settings?.providers.length ? (
                  <Button variant="secondary" size="sm" onClick={startNew}>
                    {t('addModel')}
                  </Button>
                ) : (
                  <Button variant="secondary" size="sm" onClick={() => startProviderWizard()}>
                    {t('addProvider')}
                  </Button>
                )
              }
            />
          ) : (
            <ul>
              {filtered.map((model) => (
                <li key={model.id}>
                  <div className="cd-model-manager__item-top">
                    <div>
                      <strong dir="auto">{d(model.displayName)}</strong>
                      <small dir="ltr">{model.modelId}</small>
                    </div>
                    <span>{d(model.providerName)}</span>
                  </div>
                  <div className="cd-model-manager__badges">
                    {model.isDefault ? <Badge tone="accent">{t('default')}</Badge> : null}
                    {model.supportsTools ? <Badge tone="muted"><Wrench size={11} /> {t('supportsTools')}</Badge> : null}
                    {model.supportsVision ? <Badge tone="muted"><Eye size={11} /> {t('supportsVision')}</Badge> : null}
                    {model.contextWindow ? <Badge tone="muted">{model.contextWindow.toLocaleString()}</Badge> : null}
                  </div>
                  <div className="cd-model-manager__actions">
                    <IconButton label={t('edit')} size="sm" onClick={() => setEditing(inputFromModel(model))}>
                      <Pencil size={13} />
                    </IconButton>
                    <IconButton
                      label={t('setDefault')}
                      size="sm"
                      onClick={() => void setDefaultModel(model.id)}
                      active={model.isDefault}
                    >
                      <Star size={13} />
                    </IconButton>
                    <IconButton label={t('delete')} size="sm" onClick={() => void deleteModel(model.id)}>
                      <Trash2 size={13} />
                    </IconButton>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="cd-model-manager__editor">
          {editing ? (
            <>
              <h3>{editing.id ? t('edit') : t('addModel')}</h3>
              <div className="cd-form-grid">
                <label className="cd-field cd-field--full">
                  <span>{t('addProvider')}</span>
                  <select
                    value={editing.providerId}
                    onChange={(event) => setEditing({ ...editing, providerId: event.target.value })}
                  >
                    {settings?.providers.map((provider) => (
                      <option key={provider.id} value={provider.id}>
                        {d(provider.displayName)}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="cd-field">
                  <span>{t('modelId')}</span>
                  <input
                    value={editing.modelId}
                    onChange={(event) => setEditing({ ...editing, modelId: event.target.value })}
                    dir="ltr"
                  />
                </label>
                <label className="cd-field">
                  <span>{t('displayName')}</span>
                  <input
                    value={editing.displayName}
                    onChange={(event) => setEditing({ ...editing, displayName: event.target.value })}
                    dir="auto"
                  />
                </label>
                <label className="cd-field">
                  <span>{t('contextWindow')}</span>
                  <input
                    type="number"
                    min={0}
                    value={editing.contextWindow ?? ''}
                    onChange={(event) =>
                      setEditing({
                        ...editing,
                        contextWindow: event.target.value ? Number(event.target.value) : null,
                      })
                    }
                    dir="ltr"
                  />
                </label>
                <label className="cd-mini-check">
                  <input
                    type="checkbox"
                    checked={editing.supportsTools}
                    onChange={(event) => setEditing({ ...editing, supportsTools: event.target.checked })}
                  />
                  {t('supportsTools')}
                </label>
                <label className="cd-mini-check">
                  <input
                    type="checkbox"
                    checked={editing.supportsVision}
                    onChange={(event) => setEditing({ ...editing, supportsVision: event.target.checked })}
                  />
                  {t('supportsVision')}
                </label>
                <label className="cd-mini-check">
                  <input
                    type="checkbox"
                    checked={editing.setDefault ?? false}
                    onChange={(event) => setEditing({ ...editing, setDefault: event.target.checked })}
                  />
                  {t('setDefault')}
                </label>
              </div>
              <div className="cd-model-manager__editor-actions">
                <Button variant="ghost" onClick={() => setEditing(null)}>
                  {t('cancel')}
                </Button>
                <Button
                  variant="primary"
                  onClick={() => void saveEditing()}
                  loading={providerBusy}
                  disabled={!editing.providerId || !editing.modelId.trim()}
                >
                  {t('save')}
                </Button>
              </div>
            </>
          ) : (
            <EmptyState
              icon={<Star size={24} />}
              title={t('modelSelector')}
              description={t('selectModels')}
              compact
            />
          )}
        </div>
      </div>
    </Modal>
  );
}
