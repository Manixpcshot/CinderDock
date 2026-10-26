import { useEffect, useMemo, useRef, useState } from 'react';
import { Check, ChevronDown, Eye, Search, Settings2, Wrench } from 'lucide-react';
import { useApp } from '../state/AppContext';
import { classNames } from '../lib/format';
import { Badge } from './Badge';
import { Button } from './Button';

export function ModelSelector({ compact = false }: { compact?: boolean }) {
  const {
    t,
    d,
    settings,
    selectedModelId,
    changeModel,
    openModal,
    modelQuery,
    setModelQuery,
  } = useApp();
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  const models = useMemo(() => settings?.models ?? [], [settings?.models]);
  const current = models.find((model) => model.id === selectedModelId);
  const query = modelQuery.trim().toLowerCase();
  const filtered = useMemo(
    () =>
      models.filter((model) =>
        `${model.displayName} ${model.modelId} ${model.providerName}`
          .toLowerCase()
          .includes(query),
      ),
    [models, query],
  );

  useEffect(() => {
    if (!open) return;
    const timer = window.setTimeout(() => searchRef.current?.focus(), 30);
    function onPointerDown(event: MouseEvent) {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener('mousedown', onPointerDown);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('mousedown', onPointerDown);
    };
  }, [open]);

  useEffect(() => setActiveIndex(0), [query, open]);

  function choose(index: number) {
    const model = filtered[index];
    if (!model) return;
    void changeModel(model.id);
    setOpen(false);
  }

  return (
    <div ref={rootRef} className={classNames('cd-model-selector', compact && 'is-compact')}>
      <button
        type="button"
        className="cd-model-selector__trigger"
        onClick={() => setOpen((value) => !value)}
        aria-haspopup="listbox"
        aria-expanded={open}
        title={current ? `${current.displayName} — ${current.modelId}` : t('noModelSelected')}
      >
        <span className="cd-model-selector__dot" />
        <span className="cd-model-selector__label" dir="auto">
          {current ? d(current.displayName) : t('noModelSelected')}
        </span>
        {current?.supportsVision ? <Eye size={13} aria-label={t('supportsVision')} /> : null}
        {current?.supportsTools ? <Wrench size={13} aria-label={t('supportsTools')} /> : null}
        <ChevronDown size={14} />
      </button>
      {open ? (
        <div className="cd-model-selector__popover" role="dialog" aria-label={t('modelSelector')}>
          <div className="cd-model-selector__search">
            <Search size={15} />
            <input
              ref={searchRef}
              value={modelQuery}
              onChange={(event) => setModelQuery(event.target.value)}
              placeholder={t('searchProviderOrModel')}
              aria-label={t('searchProviderOrModel')}
              onKeyDown={(event) => {
                if (event.key === 'ArrowDown') {
                  event.preventDefault();
                  setActiveIndex((value) => Math.min(value + 1, Math.max(0, filtered.length - 1)));
                } else if (event.key === 'ArrowUp') {
                  event.preventDefault();
                  setActiveIndex((value) => Math.max(value - 1, 0));
                } else if (event.key === 'Enter') {
                  event.preventDefault();
                  choose(activeIndex);
                } else if (event.key === 'Escape') {
                  event.preventDefault();
                  setOpen(false);
                }
              }}
            />
          </div>
          <div className="cd-model-selector__list" role="listbox">
            {filtered.length === 0 ? (
              <div className="cd-model-selector__empty">{t('noModels')}</div>
            ) : (
              filtered.map((model, index) => (
                <button
                  key={model.id}
                  type="button"
                  role="option"
                  aria-selected={model.id === selectedModelId}
                  className={classNames(
                    'cd-model-option',
                    model.id === selectedModelId && 'is-selected',
                    index === activeIndex && 'is-active',
                  )}
                  onMouseEnter={() => setActiveIndex(index)}
                  onClick={() => choose(index)}
                >
                  <div>
                    <strong dir="auto">{d(model.displayName)}</strong>
                    <small dir="ltr">{model.modelId}</small>
                  </div>
                  <div className="cd-model-option__meta">
                    <span>{d(model.providerName)}</span>
                    {model.isDefault ? <Badge tone="accent">{t('default')}</Badge> : null}
                    {model.supportsTools ? <Wrench size={13} /> : null}
                    {model.supportsVision ? <Eye size={13} /> : null}
                    {model.id === selectedModelId ? <Check size={15} /> : null}
                  </div>
                </button>
              ))
            )}
          </div>
          <Button
            variant="quiet"
            size="sm"
            icon={<Settings2 size={14} />}
            onClick={() => {
              setOpen(false);
              openModal('models');
            }}
          >
            {t('manageModels')}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
