import { useEffect, useMemo, useState } from 'react';
import {
  CheckCircle2,
  Download,
  KeyRound,
  Plus,
  RefreshCw,
  ShieldCheck,
  Trash2,
  XCircle,
} from 'lucide-react';
import { useApp } from '../state/AppContext';
import type {
  ModelInput,
  ProviderInput,
  ProviderPreset,
  RemoteModel,
} from '../lib/types';
import { Badge } from './Badge';
import { Button, IconButton } from './Button';
import { Modal } from './Modal';

type Step = 'provider' | 'models';

interface RemoteSelection {
  selected: boolean;
  displayName: string;
  supportsTools: boolean;
  supportsVision: boolean;
  setDefault: boolean;
}

function draftFromPreset(preset: ProviderPreset): ProviderInput {
  return {
    id: null,
    displayName: preset.label,
    preset: preset.id,
    kind: preset.kind,
    baseUrl: preset.baseUrl,
    authMethod: preset.authMethod,
    authHeader: null,
    apiKey: null,
    extraHeaders: {},
    modelsPath: preset.modelsPath,
    chatPath: preset.chatPath,
  };
}

export function ProviderDialog() {
  const {
    t,
    d,
    closeModal,
    presets,
    settings,
    providerDraft,
    setProviderDraft,
    editingProviderId,
    remoteModels,
    setRemoteModels,
    providerStatuses,
    providerBusy,
    providerError,
    setProviderError,
    testProvider,
    testSavedProvider,
    saveProvider,
    deleteProvider,
    fetchModels,
    saveModels,
    startProviderWizard,
    currentModel,
  } = useApp();
  const [step, setStep] = useState<Step>('provider');
  const [headerName, setHeaderName] = useState('');
  const [headerValue, setHeaderValue] = useState('');
  const [selections, setSelections] = useState<Record<string, RemoteSelection>>({});
  const [manual, setManual] = useState({
    modelId: '',
    displayName: '',
    supportsTools: false,
    supportsVision: false,
    setDefault: false,
  });
  const [savedProviderId, setSavedProviderId] = useState<string | null>(editingProviderId);

  useEffect(() => {
    if (!providerDraft) startProviderWizard();
  }, [providerDraft, startProviderWizard]);

  useEffect(() => {
    setSavedProviderId(editingProviderId);
  }, [editingProviderId]);

  useEffect(() => {
    setSelections((current) => {
      const next: Record<string, RemoteSelection> = {};
      for (const remote of remoteModels) {
        next[remote.id] = current[remote.id] ?? {
          selected: false,
          displayName: remote.displayName,
          supportsTools: remote.supportsTools ?? false,
          supportsVision: remote.supportsVision ?? false,
          setDefault: false,
        };
      }
      return next;
    });
  }, [remoteModels]);

  const draft = providerDraft;
  const preset = presets.find((item) => item.id === draft?.preset);
  const status = draft
    ? providerStatuses[draft.id ?? '__draft__']
    : undefined;
  const providerId = savedProviderId ?? draft?.id ?? null;

  const modelInputs = useMemo(() => {
    if (!providerId) return [];
    const inputs: ModelInput[] = [];
    for (const remote of remoteModels as RemoteModel[]) {
      const selection = selections[remote.id];
      if (!selection?.selected) continue;
      inputs.push({
        providerId,
        modelId: remote.id,
        displayName: selection.displayName || remote.id,
        supportsTools: selection.supportsTools,
        supportsVision: selection.supportsVision,
        contextWindow: remote.contextWindow ?? null,
        setDefault: selection.setDefault,
      });
    }
    if (manual.modelId.trim()) {
      inputs.push({
        providerId,
        modelId: manual.modelId.trim(),
        displayName: manual.displayName.trim() || manual.modelId.trim(),
        supportsTools: manual.supportsTools,
        supportsVision: manual.supportsVision,
        contextWindow: null,
        setDefault: manual.setDefault,
      });
    }
    return inputs;
  }, [manual, providerId, remoteModels, selections]);

  function updateDraft(patch: Partial<ProviderInput>) {
    if (!draft) return;
    setProviderDraft({ ...draft, ...patch });
  }

  function addHeader() {
    if (!draft || !headerName.trim()) return;
    updateDraft({
      extraHeaders: { ...draft.extraHeaders, [headerName.trim()]: headerValue },
    });
    setHeaderName('');
    setHeaderValue('');
  }

  async function saveAndContinue() {
    if (!draft) return;
    const saved = await saveProvider(draft);
    if (!saved) return;
    setSavedProviderId(saved.id);
    setStep('models');
    await fetchModels(saved.id);
  }

  async function saveSelectedModels() {
    if (modelInputs.length === 0) {
      setProviderError(t('noRemoteModels'));
      return;
    }
    const result = await saveModels(modelInputs);
    if (result) closeModal();
  }

  return (
    <Modal
      title={t('providers')}
      subtitle={t('addProviderHint')}
      onClose={closeModal}
      size="xl"
      footer={
        <>
          <Button variant="ghost" onClick={closeModal}>
            {t('cancel')}
          </Button>
          <div className="cd-modal__footer-spacer" />
          {step === 'provider' ? (
            <>
              <Button
                variant="secondary"
                onClick={() => { if (draft) void testProvider(draft); }}
                loading={providerBusy}
                disabled={!draft}
              >
                {t('testConnection')}
              </Button>
              <Button
                variant="primary"
                onClick={() => void saveAndContinue()}
                loading={providerBusy}
                disabled={!draft}
              >
                {t('saveAndContinue')}
              </Button>
            </>
          ) : (
            <>
              <Button
                variant="secondary"
                icon={<RefreshCw size={14} />}
                onClick={() => providerId && void fetchModels(providerId)}
                loading={providerBusy}
                disabled={!providerId}
              >
                {t('refresh')}
              </Button>
              <Button
                variant="primary"
                icon={<CheckCircle2 size={14} />}
                onClick={() => void saveSelectedModels()}
                loading={providerBusy}
                disabled={modelInputs.length === 0}
              >
                {t('save')} ({modelInputs.length})
              </Button>
            </>
          )}
        </>
      }
    >
      <div className="cd-provider-layout">
        <aside className="cd-provider-list">
          <div className="cd-provider-list__header">
            <strong>{t('providerStatus')}</strong>
            <Button
              variant="quiet"
              size="sm"
              icon={<Plus size={13} />}
              onClick={() => {
                setStep('provider');
                setSavedProviderId(null);
                setRemoteModels([]);
                startProviderWizard();
              }}
            >
              {t('addProvider')}
            </Button>
          </div>
          {settings?.providers.length ? (
            <ul>
              {settings.providers.map((provider) => {
                const providerStatus = providerStatuses[provider.id];
                return (
                  <li key={provider.id} className={providerId === provider.id ? 'is-active' : ''}>
                    <button
                      type="button"
                      onClick={() => {
                        setStep('provider');
                        startProviderWizard(provider.preset, provider);
                      }}
                    >
                      <strong dir="auto">{d(provider.displayName)}</strong>
                      <small dir="ltr">{provider.baseUrl}</small>
                      <span>
                        <Badge tone="muted">{provider.modelCount} {t('model')}</Badge>
                        {provider.hasSecret ? <Badge tone="success"><KeyRound size={11} /></Badge> : null}
                        {providerStatus ? (
                          <Badge tone={providerStatus.ok ? 'success' : 'danger'}>
                            {providerStatus.ok ? t('online') : t('offline')}
                          </Badge>
                        ) : null}
                      </span>
                    </button>
                    <div className="cd-provider-list__actions">
                      <IconButton
                        label={t('test')}
                        size="sm"
                        onClick={() => void testSavedProvider(provider.id)}
                      >
                        <RefreshCw size={13} />
                      </IconButton>
                      <IconButton
                        label={t('delete')}
                        size="sm"
                        onClick={() => void deleteProvider(provider.id)}
                      >
                        <Trash2 size={13} />
                      </IconButton>
                    </div>
                  </li>
                );
              })}
            </ul>
          ) : (
            <p>{t('noProviders')}</p>
          )}
          <div className="cd-provider-list__secure">
            <ShieldCheck size={16} />
            <span>{t('apiKeyWarning')}</span>
          </div>
        </aside>

        <div className="cd-provider-main">
          {step === 'provider' ? (
            draft ? (
              <div className="cd-form-grid">
                <label className="cd-field cd-field--full">
                  <span>{t('preset')}</span>
                  <select
                    value={draft.preset}
                    onChange={(event) => {
                      const next = presets.find((item) => item.id === event.target.value);
                      if (next) {
                        setProviderDraft(draftFromPreset(next));
                        setSavedProviderId(null);
                      }
                    }}
                  >
                    {presets.map((item) => (
                      <option key={item.id} value={item.id}>
                        {d(item.label)}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="cd-field">
                  <span>{t('providerName')}</span>
                  <input
                    value={draft.displayName}
                    onChange={(event) => updateDraft({ displayName: event.target.value })}
                    dir="auto"
                  />
                </label>
                <label className="cd-field">
                  <span>{t('baseUrl')}</span>
                  <input
                    value={draft.baseUrl}
                    onChange={(event) => updateDraft({ baseUrl: event.target.value })}
                    dir="ltr"
                    spellCheck={false}
                  />
                </label>
                <label className="cd-field">
                  <span>{t('authMethod')}</span>
                  <select
                    value={draft.authMethod}
                    onChange={(event) =>
                      updateDraft({ authMethod: event.target.value as ProviderInput['authMethod'] })
                    }
                  >
                    <option value="bearer">{t('bearer')}</option>
                    <option value="none">{t('none')}</option>
                    <option value="customHeader">{t('customHeader')}</option>
                  </select>
                </label>
                <label className="cd-field">
                  <span>{t('apiKey')}</span>
                  <input
                    type="password"
                    value={draft.apiKey ?? ''}
                    placeholder={savedProviderId ? t('replaceKey') : ''}
                    onChange={(event) => updateDraft({ apiKey: event.target.value })}
                    autoComplete="off"
                    spellCheck={false}
                    dir="ltr"
                  />
                  {savedProviderId && settings?.providers.find((item) => item.id === savedProviderId)?.hasSecret ? (
                    <small>{t('savedKey')}</small>
                  ) : null}
                </label>
                {draft.authMethod === 'customHeader' ? (
                  <label className="cd-field">
                    <span>{t('authHeader')}</span>
                    <input
                      value={draft.authHeader ?? ''}
                      onChange={(event) => updateDraft({ authHeader: event.target.value })}
                      dir="ltr"
                      placeholder="X-API-Key"
                    />
                  </label>
                ) : null}
                <label className="cd-field">
                  <span>{t('modelsPath')}</span>
                  <input
                    value={draft.modelsPath ?? ''}
                    onChange={(event) => updateDraft({ modelsPath: event.target.value })}
                    dir="ltr"
                  />
                </label>
                <label className="cd-field">
                  <span>{t('chatPath')}</span>
                  <input
                    value={draft.chatPath ?? ''}
                    onChange={(event) => updateDraft({ chatPath: event.target.value })}
                    dir="ltr"
                  />
                </label>
                <div className="cd-field cd-field--full">
                  <span>{t('optionalHeaders')}</span>
                  <div className="cd-header-editor">
                    <input
                      value={headerName}
                      onChange={(event) => setHeaderName(event.target.value)}
                      placeholder={t('headerName')}
                      dir="ltr"
                    />
                    <input
                      value={headerValue}
                      onChange={(event) => setHeaderValue(event.target.value)}
                      placeholder={t('headerValue')}
                      dir="ltr"
                    />
                    <Button variant="secondary" size="sm" icon={<Plus size={13} />} onClick={addHeader}>
                      {t('addHeader')}
                    </Button>
                  </div>
                  <div className="cd-header-chips">
                    {Object.entries(draft.extraHeaders).map(([name, value]) => (
                      <span key={name}>
                        <code dir="ltr">{name}: {value}</code>
                        <button
                          type="button"
                          onClick={() => {
                            const next = { ...draft.extraHeaders };
                            delete next[name];
                            updateDraft({ extraHeaders: next });
                          }}
                          aria-label={`${t('delete')} ${name}`}
                        >
                          ×
                        </button>
                      </span>
                    ))}
                  </div>
                </div>
                <div className="cd-field cd-field--full">
                  <div className="cd-provider-note">
                    <Badge tone={draft.kind === 'ollamaNative' ? 'accent' : 'muted'}>
                      {draft.kind === 'ollamaNative' ? t('ollamaNative') : t('openAICompatible')}
                    </Badge>
                    <span dir="auto">{d(preset?.description ?? '')} — {d(preset?.notes ?? '')}</span>
                  </div>
                </div>
                {status ? (
                  <div className={`cd-test-result cd-test-result--${status.ok ? 'ok' : 'bad'}`}>
                    {status.ok ? <CheckCircle2 size={16} /> : <XCircle size={16} />}
                    <span dir="auto">{d(status.message)}</span>
                    <Badge tone="muted">{status.latencyMs} ms</Badge>
                    {status.modelCount !== null && status.modelCount !== undefined ? (
                      <Badge tone="accent">{status.modelCount} {t('model')}</Badge>
                    ) : null}
                  </div>
                ) : null}
                {providerError ? <div className="cd-form-error">{d(providerError)}</div> : null}
                <div className="cd-field cd-field--full cd-security-note">
                  <KeyRound size={15} />
                  {t('keyStoredSecurely')}
                </div>
              </div>
            ) : null
          ) : (
            <div className="cd-model-import">
              <div className="cd-model-import__header">
                <div>
                  <h3>{t('selectModels')}</h3>
                  <p>{t('modelDiscoveryUnavailable')}</p>
                </div>
                <Button
                  variant="secondary"
                  size="sm"
                  icon={<Download size={14} />}
                  onClick={() => providerId && void fetchModels(providerId)}
                  loading={providerBusy}
                >
                  {t('fetchModels')}
                </Button>
              </div>
              {providerError ? <div className="cd-form-error">{d(providerError)}</div> : null}
              {remoteModels.length > 0 ? (
                <div className="cd-remote-models">
                  {remoteModels.map((remote) => {
                    const selection = selections[remote.id];
                    return (
                      <div key={remote.id} className="cd-remote-model">
                        <label className="cd-checkbox">
                          <input
                            type="checkbox"
                            checked={selection?.selected ?? false}
                            onChange={(event) =>
                              setSelections((current) => ({
                                ...current,
                                [remote.id]: {
                                  ...(current[remote.id] ?? {
                                    displayName: remote.displayName,
                                    supportsTools: remote.supportsTools ?? false,
                                    supportsVision: remote.supportsVision ?? false,
                                    setDefault: false,
                                  }),
                                  selected: event.target.checked,
                                },
                              }))
                            }
                          />
                          <div>
                            <strong dir="ltr">{remote.id}</strong>
                            <small dir="auto">{d(remote.displayName)}</small>
                          </div>
                        </label>
                        {remote.supportsTools === null || remote.supportsTools === undefined ? (
                          <Badge tone="warning">{t('capabilityUnknown')}</Badge>
                        ) : null}
                        <label className="cd-mini-check">
                          <input
                            type="checkbox"
                            checked={selection?.supportsTools ?? false}
                            onChange={(event) =>
                              setSelections((current) => ({
                                ...current,
                                [remote.id]: {
                                  ...(current[remote.id] ?? {
                                    selected: false,
                                    displayName: remote.displayName,
                                    supportsVision: false,
                                    setDefault: false,
                                  }),
                                  supportsTools: event.target.checked,
                                },
                              }))
                            }
                          />
                          {t('supportsTools')}
                        </label>
                        <label className="cd-mini-check">
                          <input
                            type="checkbox"
                            checked={selection?.supportsVision ?? false}
                            onChange={(event) =>
                              setSelections((current) => ({
                                ...current,
                                [remote.id]: {
                                  ...(current[remote.id] ?? {
                                    selected: false,
                                    displayName: remote.displayName,
                                    supportsTools: false,
                                    setDefault: false,
                                  }),
                                  supportsVision: event.target.checked,
                                },
                              }))
                            }
                          />
                          {t('supportsVision')}
                        </label>
                        <label className="cd-mini-check">
                          <input
                            type="checkbox"
                            checked={selection?.setDefault ?? false}
                            onChange={(event) =>
                              setSelections((current) => ({
                                ...current,
                                [remote.id]: {
                                  ...(current[remote.id] ?? {
                                    selected: false,
                                    displayName: remote.displayName,
                                    supportsTools: false,
                                    supportsVision: false,
                                  }),
                                  setDefault: event.target.checked,
                                },
                              }))
                            }
                          />
                          {t('setDefault')}
                        </label>
                      </div>
                    );
                  })}
                </div>
              ) : (
                <p className="cd-provider-empty">{t('noRemoteModels')}</p>
              )}
              <section className="cd-manual-model">
                <h3>{t('manualModel')}</h3>
                <div className="cd-form-grid">
                  <label className="cd-field">
                    <span>{t('modelId')}</span>
                    <input
                      value={manual.modelId}
                      onChange={(event) => setManual({ ...manual, modelId: event.target.value })}
                      dir="ltr"
                      placeholder="gpt-4o / qwen2.5-coder / …"
                    />
                  </label>
                  <label className="cd-field">
                    <span>{t('displayName')}</span>
                    <input
                      value={manual.displayName}
                      onChange={(event) => setManual({ ...manual, displayName: event.target.value })}
                      dir="auto"
                    />
                  </label>
                  <label className="cd-mini-check">
                    <input
                      type="checkbox"
                      checked={manual.supportsTools}
                      onChange={(event) => setManual({ ...manual, supportsTools: event.target.checked })}
                    />
                    {t('supportsTools')}
                  </label>
                  <label className="cd-mini-check">
                    <input
                      type="checkbox"
                      checked={manual.supportsVision}
                      onChange={(event) => setManual({ ...manual, supportsVision: event.target.checked })}
                    />
                    {t('supportsVision')}
                  </label>
                  <label className="cd-mini-check">
                    <input
                      type="checkbox"
                      checked={manual.setDefault}
                      onChange={(event) => setManual({ ...manual, setDefault: event.target.checked })}
                    />
                    {t('setDefault')}
                  </label>
                </div>
              </section>
              {currentModel ? (
                <p className="cd-model-import__current">
                  {t('defaultModel')}: <strong dir="auto">{d(currentModel.displayName)}</strong>
                </p>
              ) : null}
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
