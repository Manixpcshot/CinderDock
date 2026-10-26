import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { createTranslator } from '../lib/i18n';
import { invokeCommand, subscribeAgentEvents } from '../lib/tauri';
import type {
  AgentEvent,
  ApprovalDecisionValue,
  ApprovalRequest,
  AttachmentInput,
  Bootstrap,
  ConnectionState,
  ConversationData,
  ConversationSummary,
  FileChangeView,
  InspectorTab,
  MessageView,
  ModalType,
  ModelInput,
  ModelView,
  Preferences,
  ProjectInfo,
  ProviderInput,
  ProviderPreset,
  ProviderView,
  RemoteModel,
  RunStart,
  SettingsView,
  TerminalLine,
  TestConnectionResult,
  ToastState,
  ToolEventView,
} from '../lib/types';

const defaultPreferences: Preferences = {
  theme: 'ember-dark',
  uiLanguage: 'fa',
  textMode: 'native',
  fontScale: 1,
  desktopToolsEnabled: false,
  webAccessEnabled: true,
  maxToolIterations: 8,
  maxRetries: 2,
  terminalVisible: false,
  defaultModelId: null,
};

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return typeof error === 'string' ? error : 'خطای ناشناخته';
}

function isImagePath(path: string): boolean {
  return /\.(png|jpe?g|webp|gif|bmp)$/i.test(path);
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

function draftFromProvider(provider: ProviderView): ProviderInput {
  return {
    id: provider.id,
    displayName: provider.displayName,
    preset: provider.preset,
    kind: provider.kind,
    baseUrl: provider.baseUrl,
    authMethod: provider.authMethod,
    authHeader: provider.authHeader,
    apiKey: null,
    extraHeaders: { ...provider.extraHeaders },
    modelsPath: provider.modelsPath,
    chatPath: provider.chatPath,
  };
}

function upsertById<T extends { id: string }>(items: T[], item: T): T[] {
  const index = items.findIndex((candidate) => candidate.id === item.id);
  if (index === -1) return [...items, item];
  const next = [...items];
  next[index] = item;
  return next;
}

function useCinderDockState() {
  const [loading, setLoading] = useState(true);
  const [bootError, setBootError] = useState<string | null>(null);
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [preferences, setPreferences] = useState<Preferences>(defaultPreferences);
  const [presets, setPresets] = useState<ProviderPreset[]>([]);
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [projectFiles, setProjectFiles] = useState<string[]>([]);
  const [activeConversationId, setActiveConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageView[]>([]);
  const [toolEvents, setToolEvents] = useState<ToolEventView[]>([]);
  const [fileChanges, setFileChanges] = useState<FileChangeView[]>([]);
  const [terminalLines, setTerminalLines] = useState<TerminalLine[]>([]);
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [composer, setComposer] = useState('');
  const [attachments, setAttachments] = useState<AttachmentInput[]>([]);
  const [streaming, setStreaming] = useState(false);
  const [streamingMessageId, setStreamingMessageId] = useState<string | null>(null);
  const [streamingContent, setStreamingContent] = useState('');
  const [runId, setRunId] = useState<string | null>(null);
  const [status, setStatus] = useState('idle');
  const [statusMessage, setStatusMessage] = useState('');
  const [connection, setConnection] = useState<ConnectionState>({
    state: 'unknown',
    message: '',
  });
  const [approval, setApproval] = useState<ApprovalRequest | null>(null);
  const [toast, setToast] = useState<ToastState | null>(null);
  const [errorBanner, setErrorBanner] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalType>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>('activity');
  const [terminalVisible, setTerminalVisible] = useState(false);
  const [modelQuery, setModelQuery] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<ConversationSummary[]>([]);
  const [providerDraft, setProviderDraft] = useState<ProviderInput | null>(null);
  const [editingProviderId, setEditingProviderId] = useState<string | null>(null);
  const [remoteModels, setRemoteModels] = useState<RemoteModel[]>([]);
  const [providerStatuses, setProviderStatuses] = useState<
    Record<string, TestConnectionResult>
  >({});
  const [providerBusy, setProviderBusy] = useState(false);
  const [providerError, setProviderError] = useState<string | null>(null);
  const streamingIdRef = useRef<string | null>(null);

  const translator = useMemo(() => createTranslator(preferences), [preferences]);
  const { t } = translator;

  const showToast = useCallback(
    (message: string, kind: ToastState['kind'] = 'info') => {
      setToast({ id: Date.now() + Math.random(), message, kind });
    },
    [],
  );

  const applySettings = useCallback((next: SettingsView) => {
    setSettings(next);
    setPreferences(next.preferences);
    setTerminalVisible(next.preferences.terminalVisible);
    setSelectedModelId((current) =>
      current && next.models.some((model) => model.id === current)
        ? current
        : next.preferences.defaultModelId,
    );
  }, []);

  const refreshSettings = useCallback(async () => {
    const next = await invokeCommand<SettingsView>('get_settings');
    applySettings(next);
    return next;
  }, [applySettings]);

  const refreshConversations = useCallback(async () => {
    const next = await invokeCommand<ConversationSummary[]>('list_conversations');
    setConversations(next);
    return next;
  }, []);

  const refreshProjectFiles = useCallback(async () => {
    const files = await invokeCommand<string[]>('list_project_files');
    setProjectFiles(files);
    return files;
  }, []);

  const refreshBootstrap = useCallback(async () => {
    setLoading(true);
    setBootError(null);
    try {
      const data = await invokeCommand<Bootstrap>('get_bootstrap');
      applySettings(data.settings);
      setPresets(data.presets);
      setConversations(data.conversations);
      await refreshProjectFiles().catch(() => setProjectFiles([]));
      setLoading(false);
      return data;
    } catch (error) {
      setBootError(errorMessage(error));
      setLoading(false);
      throw error;
    }
  }, [applySettings, refreshProjectFiles]);

  const finishStreaming = useCallback(() => {
    streamingIdRef.current = null;
    setStreaming(false);
    setStreamingMessageId(null);
    setStreamingContent('');
    setRunId(null);
    setStatus('idle');
    setStatusMessage('');
  }, []);

  const appendToolEvent = useCallback((event: ToolEventView) => {
    setToolEvents((current) =>
      current.some((candidate) => candidate.id === event.id)
        ? current
        : [...current, event],
    );
  }, []);

  const handleAgentEvent = useCallback(
    (event: AgentEvent) => {
      switch (event.type) {
        case 'status':
          setStatus(event.status);
          setStatusMessage(event.message);
          break;
        case 'messageDelta':
          if (streamingIdRef.current !== event.messageId) {
            streamingIdRef.current = event.messageId;
            setStreamingContent(event.delta);
          } else {
            setStreamingContent((current) => current + event.delta);
          }
          setStreamingMessageId(event.messageId);
          setStreaming(true);
          break;
        case 'messageDone':
          setMessages((current) => upsertById(current, event.message));
          if (streamingIdRef.current === event.message.id) finishStreaming();
          break;
        case 'toolCall':
          appendToolEvent(event.event);
          setInspectorTab('activity');
          setInspectorOpen(true);
          break;
        case 'toolResult':
        case 'toolEvent':
          appendToolEvent(event.event);
          break;
        case 'approvalRequest':
          setApproval(event.request);
          setInspectorTab('activity');
          setInspectorOpen(true);
          break;
        case 'approvalResolved': {
          setApproval((current) =>
            current?.id === event.requestId ? null : current,
          );
          appendToolEvent({
            id: `approval-${event.requestId}-${event.decision}`,
            conversationId: '',
            kind: 'approval',
            name: 'approval',
            status: event.decision,
            summary: event.fromSession ? t('sessionGrant') : t('approvalApproved'),
            details: event.decision === 'deny' ? t('approvalDenied') : t('details'),
            createdAt: new Date().toISOString(),
          });
          break;
        }
        case 'fileChange':
          setFileChanges((current) => upsertById(current, event.change));
          setInspectorTab('files');
          setInspectorOpen(true);
          break;
        case 'terminal':
          setTerminalLines((current) => [...current, ...event.lines].slice(-600));
          setTerminalVisible(true);
          break;
        case 'error': {
          setErrorBanner(event.message);
          showToast(event.message, 'error');
          const errorId = `error-${Date.now()}`;
          setMessages((current) => [
            ...current,
            {
              id: errorId,
              conversationId: event.conversationId,
              role: 'assistant',
              content: `**${t('error')}:** ${event.message}`,
              attachments: [],
              toolCalls: [],
              toolCallId: null,
              status: 'error',
              createdAt: new Date().toISOString(),
              seq: Date.now(),
            },
          ]);
          finishStreaming();
          break;
        }
        case 'done':
          finishStreaming();
          void refreshConversations().catch(() => undefined);
          break;
        case 'stopped':
          finishStreaming();
          showToast(t('operationStopped'), 'info');
          void refreshConversations().catch(() => undefined);
          break;
        case 'connection':
          setConnection({
            state: event.state,
            message: event.message,
            providerId: event.providerId,
          });
          break;
      }
    },
    [appendToolEvent, finishStreaming, refreshConversations, showToast, t],
  );

  useEffect(() => {
    void refreshBootstrap().catch(() => undefined);
  }, [refreshBootstrap]);

  useEffect(() => subscribeAgentEvents(handleAgentEvent), [handleAgentEvent]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 4500);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    const root = document.documentElement;
    root.lang = preferences.uiLanguage === 'fa' ? 'fa' : 'en';
    root.dir = translator.dir;
    root.style.setProperty('--font-scale', String(preferences.fontScale));
    root.dataset.textMode = preferences.textMode;
  }, [preferences.fontScale, preferences.textMode, preferences.uiLanguage, translator.dir]);

  const openModal = useCallback((next: ModalType) => {
    setModal(next);
    if (next === 'providers') setProviderError(null);
  }, []);

  const closeModal = useCallback(() => setModal(null), []);

  const toggleSidebar = useCallback(() => setSidebarOpen((value) => !value), []);
  const toggleInspector = useCallback(() => setInspectorOpen((value) => !value), []);
  const toggleTerminal = useCallback(() => setTerminalVisible((value) => !value), []);
  const clearTerminal = useCallback(() => setTerminalLines([]), []);
  const dismissError = useCallback(() => setErrorBanner(null), []);

  const newChat = useCallback(() => {
    setActiveConversationId(null);
    setMessages([]);
    setToolEvents([]);
    setFileChanges([]);
    setTerminalLines([]);
    setComposer('');
    setAttachments([]);
    setApproval(null);
    setErrorBanner(null);
    setModal(null);
    finishStreaming();
  }, [finishStreaming]);

  const openConversation = useCallback(
    async (conversationId: string) => {
      try {
        const data = await invokeCommand<ConversationData>('load_conversation', {
          conversationId,
        });
        setActiveConversationId(conversationId);
        setMessages(data.messages.filter((message) => message.status !== 'streaming'));
        setToolEvents(data.toolEvents);
        setFileChanges(data.fileChanges);
        setTerminalLines([]);
        setSelectedModelId(data.conversation.modelId ?? preferences.defaultModelId);
        setApproval(null);
        setErrorBanner(null);
        setModal(null);
        finishStreaming();
        if (
          data.conversation.projectPath &&
          data.conversation.projectPath !== settings?.projectPath
        ) {
          await invokeCommand<ProjectInfo>('set_project', {
            path: data.conversation.projectPath,
          });
          await refreshSettings();
          await refreshProjectFiles().catch(() => undefined);
        }
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [
      finishStreaming,
      preferences.defaultModelId,
      refreshProjectFiles,
      refreshSettings,
      settings?.projectPath,
      showToast,
    ],
  );

  const deleteConversation = useCallback(
    async (conversationId: string) => {
      if (!window.confirm(t('confirmDelete'))) return;
      try {
        const next = await invokeCommand<ConversationSummary[]>(
          'delete_conversation',
          { conversationId },
        );
        setConversations(next);
        setSearchResults((current) =>
          current.filter((conversation) => conversation.id !== conversationId),
        );
        if (activeConversationId === conversationId) newChat();
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [activeConversationId, newChat, showToast, t],
  );

  const searchConversations = useCallback(
    async (query: string) => {
      setSearchQuery(query);
      if (!query.trim()) {
        setSearchResults([]);
        await refreshConversations().catch(() => undefined);
        return;
      }
      try {
        const results = await invokeCommand<ConversationSummary[]>(
          'search_conversations',
          { query },
        );
        setSearchResults(results);
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [refreshConversations, showToast],
  );

  const chooseProject = useCallback(async () => {
    try {
      const info = await invokeCommand<ProjectInfo | null>('choose_project_folder');
      if (!info) return;
      await refreshSettings();
      await refreshProjectFiles().catch(() => undefined);
      showToast(t('projectSelected'), 'success');
    } catch (error) {
      showToast(errorMessage(error), 'error');
    }
  }, [refreshProjectFiles, refreshSettings, showToast, t]);

  const setProjectByPath = useCallback(
    async (path: string) => {
      try {
        await invokeCommand<ProjectInfo>('set_project', { path });
        await refreshSettings();
        await refreshProjectFiles().catch(() => undefined);
        showToast(t('projectSelected'), 'success');
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [refreshProjectFiles, refreshSettings, showToast, t],
  );

  const chooseAttachments = useCallback(async () => {
    try {
      const picked = await invokeCommand<AttachmentInput[]>('choose_attachments');
      if (picked.length === 0) return;
      setAttachments((current) => {
        const seen = new Set(current.map((item) => item.path));
        return [...current, ...picked.filter((item) => !seen.has(item.path))].slice(0, 8);
      });
    } catch (error) {
      showToast(errorMessage(error), 'error');
    }
  }, [showToast]);

  const removeAttachment = useCallback((path: string) => {
    setAttachments((current) => current.filter((item) => item.path !== path));
  }, []);

  const sendMessage = useCallback(async () => {
    const text = composer.trim();
    if (!text && attachments.length === 0) return;
    const model = settings?.models.find((item) => item.id === selectedModelId);
    if (!model) {
      showToast(t('addProviderFirst'), 'error');
      openModal('providers');
      return;
    }
    if (attachments.some((item) => isImagePath(item.path)) && !model.supportsVision) {
      showToast(t('visionRequired'), 'error');
      return;
    }
    try {
      setComposer('');
      setAttachments([]);
      setErrorBanner(null);
      setStreaming(true);
      setStreamingContent('');
      setStatus('connecting');
      setStatusMessage(t('connecting'));
      const start = await invokeCommand<RunStart>('send_message', {
        input: {
          conversationId: activeConversationId,
          content: text,
          attachments,
          modelId: selectedModelId,
        },
      });
      setActiveConversationId(start.conversationId);
      setRunId(start.runId);
      setStreamingMessageId(start.assistantMessageId);
      streamingIdRef.current = start.assistantMessageId;
      if (start.userMessage) {
        setMessages((current) => upsertById(current, start.userMessage as MessageView));
      }
      void refreshConversations().catch(() => undefined);
    } catch (error) {
      finishStreaming();
      showToast(errorMessage(error), 'error');
    }
  }, [
    activeConversationId,
    attachments,
    composer,
    finishStreaming,
    openModal,
    refreshConversations,
    selectedModelId,
    settings?.models,
    showToast,
    t,
  ]);

  const stopGeneration = useCallback(async () => {
    if (!runId) return;
    try {
      await invokeCommand<boolean>('stop_generation', { runId });
      showToast(t('operationStopped'), 'info');
    } catch (error) {
      showToast(errorMessage(error), 'error');
    }
  }, [runId, showToast, t]);

  const retryLast = useCallback(async () => {
    if (!activeConversationId) return;
    if (!selectedModelId) {
      showToast(t('addProviderFirst'), 'error');
      return;
    }
    try {
      setErrorBanner(null);
      setStreaming(true);
      setStreamingContent('');
      setStatus('retrying');
      const start = await invokeCommand<RunStart>('retry_last', {
        input: {
          conversationId: activeConversationId,
          modelId: selectedModelId,
        },
      });
      const data = await invokeCommand<ConversationData>('load_conversation', {
        conversationId: activeConversationId,
      });
      setMessages(data.messages.filter((message) => message.status !== 'streaming'));
      setToolEvents(data.toolEvents);
      setFileChanges(data.fileChanges);
      setRunId(start.runId);
      setStreamingMessageId(start.assistantMessageId);
      streamingIdRef.current = start.assistantMessageId;
    } catch (error) {
      finishStreaming();
      showToast(errorMessage(error), 'error');
    }
  }, [activeConversationId, finishStreaming, selectedModelId, showToast, t]);

  const respondApproval = useCallback(
    async (decision: ApprovalDecisionValue, requestId?: string) => {
      const id = requestId ?? approval?.id;
      if (!id) return;
      try {
        const accepted = await invokeCommand<boolean>('respond_approval', {
          requestId: id,
          decision,
        });
        if (!accepted) showToast(t('approvalDenied'), 'error');
        setApproval((current) => (current?.id === id ? null : current));
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [approval?.id, showToast, t],
  );

  const revertChange = useCallback(
    async (changeId: string) => {
      if (!window.confirm(t('confirmRevert'))) return;
      try {
        const change = await invokeCommand<FileChangeView>('revert_file_change', {
          changeId,
          confirmed: true,
        });
        setFileChanges((current) => upsertById(current, change));
        showToast(t('fileChangeReverted'), 'success');
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [showToast, t],
  );

  const changeModel = useCallback(
    async (modelId: string) => {
      setSelectedModelId(modelId);
      if (!activeConversationId) return;
      try {
        await invokeCommand<ConversationSummary>('set_conversation_model', {
          conversationId: activeConversationId,
          modelId,
        });
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [activeConversationId, showToast],
  );

  const startProviderWizard = useCallback(
    (presetId?: string, provider?: ProviderView | null) => {
      const preset =
        presets.find((item) => item.id === presetId) ??
        presets.find((item) => item.id === provider?.preset) ??
        presets[0];
      setProviderDraft(
        provider ? draftFromProvider(provider) : preset ? draftFromPreset(preset) : null,
      );
      setEditingProviderId(provider?.id ?? null);
      setRemoteModels([]);
      setProviderError(null);
      openModal('providers');
    },
    [openModal, presets],
  );

  const testProvider = useCallback(
    async (draft: ProviderInput) => {
      setProviderBusy(true);
      setProviderError(null);
      try {
        const result = await invokeCommand<TestConnectionResult>('test_provider', {
          input: draft,
        });
        setProviderStatuses((current) => ({
          ...current,
          [draft.id ?? '__draft__']: result,
        }));
        if (!result.ok) setProviderError(result.message);
        return result;
      } catch (error) {
        const message = errorMessage(error);
        setProviderError(message);
        showToast(message, 'error');
        return null;
      } finally {
        setProviderBusy(false);
      }
    },
    [showToast],
  );

  const testSavedProvider = useCallback(
    async (providerId: string) => {
      setProviderBusy(true);
      try {
        const result = await invokeCommand<TestConnectionResult>(
          'test_saved_provider',
          { providerId },
        );
        setProviderStatuses((current) => ({ ...current, [providerId]: result }));
        if (!result.ok) showToast(result.message, 'error');
        return result;
      } catch (error) {
        showToast(errorMessage(error), 'error');
        return null;
      } finally {
        setProviderBusy(false);
      }
    },
    [showToast],
  );

  const saveProvider = useCallback(
    async (draft: ProviderInput) => {
      setProviderBusy(true);
      setProviderError(null);
      try {
        const view = await invokeCommand<ProviderView>('save_provider', {
          input: draft,
        });
        await refreshSettings();
        setProviderDraft({ ...draft, id: view.id, apiKey: null });
        setEditingProviderId(view.id);
        showToast(t('providerSaved'), 'success');
        return view;
      } catch (error) {
        const message = errorMessage(error);
        setProviderError(message);
        showToast(message, 'error');
        return null;
      } finally {
        setProviderBusy(false);
      }
    },
    [refreshSettings, showToast, t],
  );

  const deleteProvider = useCallback(
    async (providerId: string) => {
      if (!window.confirm(t('removeProvider') + '?')) return;
      try {
        const next = await invokeCommand<SettingsView>('delete_provider', {
          providerId,
        });
        applySettings(next);
        if (editingProviderId === providerId) {
          setProviderDraft(null);
          setEditingProviderId(null);
          setRemoteModels([]);
        }
        showToast(t('removeProvider'), 'success');
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [applySettings, editingProviderId, showToast, t],
  );

  const fetchModels = useCallback(
    async (providerId: string) => {
      setProviderBusy(true);
      setProviderError(null);
      try {
        const models = await invokeCommand<RemoteModel[]>(
          'fetch_provider_models',
          { providerId },
        );
        setRemoteModels(models);
        if (models.length === 0) setProviderError(t('noRemoteModels'));
        return models;
      } catch (error) {
        const message = errorMessage(error);
        setProviderError(message);
        showToast(message, 'error');
        return [];
      } finally {
        setProviderBusy(false);
      }
    },
    [showToast, t],
  );

  const saveModels = useCallback(
    async (inputs: ModelInput[]) => {
      setProviderBusy(true);
      try {
        const next = await invokeCommand<SettingsView>('save_models', { inputs });
        applySettings(next);
        showToast(t('modelSaved'), 'success');
        return next;
      } catch (error) {
        showToast(errorMessage(error), 'error');
        return null;
      } finally {
        setProviderBusy(false);
      }
    },
    [applySettings, showToast, t],
  );

  const deleteModel = useCallback(
    async (modelId: string) => {
      if (!window.confirm(t('removeModel') + '?')) return;
      try {
        const next = await invokeCommand<SettingsView>('delete_model', { modelId });
        applySettings(next);
        showToast(t('removeModel'), 'success');
      } catch (error) {
        showToast(errorMessage(error), 'error');
      }
    },
    [applySettings, showToast, t],
  );

  const setDefaultModel = useCallback(
    async (modelId: string | null) => {
      try {
        const next = await invokeCommand<SettingsView>('set_default_model', {
          modelId,
        });
        applySettings(next);
        if (modelId) setSelectedModelId(modelId);
        return next;
      } catch (error) {
        showToast(errorMessage(error), 'error');
        return null;
      }
    },
    [applySettings, showToast],
  );

  const updatePreferences = useCallback(
    async (patch: Preferences) => {
      try {
        const next = await invokeCommand<SettingsView>('update_preferences', {
          patch,
        });
        applySettings(next);
        showToast(t('settingsSaved'), 'success');
        return next;
      } catch (error) {
        showToast(errorMessage(error), 'error');
        return null;
      }
    },
    [applySettings, showToast, t],
  );

  const clearSessionGrants = useCallback(async () => {
    try {
      await invokeCommand<boolean>('clear_session_grants');
      showToast(t('sessionGrantsCleared'), 'success');
    } catch (error) {
      showToast(errorMessage(error), 'error');
    }
  }, [showToast, t]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const modifier = event.ctrlKey || event.metaKey;
      if (modifier && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        openModal('search');
      } else if (modifier && event.key.toLowerCase() === 'n') {
        event.preventDefault();
        newChat();
      } else if (modifier && event.key === ',') {
        event.preventDefault();
        openModal('settings');
      } else if (modifier && event.key.toLowerCase() === 'b') {
        event.preventDefault();
        toggleSidebar();
      } else if (modifier && event.shiftKey && event.key.toLowerCase() === 't') {
        event.preventDefault();
        toggleTerminal();
      } else if (event.altKey && ['1', '2', '3'].includes(event.key)) {
        event.preventDefault();
        setInspectorTab(
          event.key === '1' ? 'activity' : event.key === '2' ? 'files' : 'terminal',
        );
      } else if (event.key === 'Escape' && modal && !approval) {
        closeModal();
      }
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [
    approval,
    closeModal,
    modal,
    newChat,
    openModal,
    toggleSidebar,
    toggleTerminal,
  ]);

  const currentModel: ModelView | undefined = useMemo(
    () => settings?.models.find((model) => model.id === selectedModelId),
    [selectedModelId, settings?.models],
  );

  const currentProvider = useMemo(
    () => settings?.providers.find((provider) => provider.id === currentModel?.providerId),
    [currentModel?.providerId, settings?.providers],
  );

  const recentProjects = useMemo(
    () => settings?.recentProjects ?? [],
    [settings?.recentProjects],
  );

  const visionWarning = useMemo(
    () =>
      attachments.some((item) => isImagePath(item.path)) &&
      Boolean(currentModel) &&
      !currentModel?.supportsVision,
    [attachments, currentModel],
  );

  return {
    ...translator,
    loading,
    bootError,
    settings,
    preferences,
    presets,
    recentProjects,
    conversations,
    projectFiles,
    activeConversationId,
    messages,
    toolEvents,
    fileChanges,
    terminalLines,
    selectedModelId,
    currentModel,
    currentProvider,
    composer,
    attachments,
    streaming,
    streamingMessageId,
    streamingContent,
    runId,
    status,
    statusMessage,
    connection,
    approval,
    toast,
    errorBanner,
    modal,
    sidebarOpen,
    inspectorOpen,
    inspectorTab,
    terminalVisible,
    modelQuery,
    searchQuery,
    searchResults,
    providerDraft,
    editingProviderId,
    remoteModels,
    providerStatuses,
    providerBusy,
    providerError,
    visionWarning,
    showToast,
    setToast,
    refreshBootstrap,
    refreshSettings,
    refreshConversations,
    refreshProjectFiles,
    openModal,
    closeModal,
    setModal,
    toggleSidebar,
    toggleInspector,
    setInspectorOpen,
    setInspectorTab,
    toggleTerminal,
    setTerminalVisible,
    clearTerminal,
    dismissError,
    newChat,
    openConversation,
    deleteConversation,
    searchConversations,
    chooseProject,
    setProjectByPath,
    chooseAttachments,
    removeAttachment,
    setComposer,
    setAttachments,
    sendMessage,
    stopGeneration,
    retryLast,
    respondApproval,
    revertChange,
    changeModel,
    startProviderWizard,
    setProviderDraft,
    setEditingProviderId,
    setRemoteModels,
    setProviderError,
    setModelQuery,
    setSearchQuery,
    testProvider,
    testSavedProvider,
    saveProvider,
    deleteProvider,
    fetchModels,
    saveModels,
    deleteModel,
    setDefaultModel,
    updatePreferences,
    clearSessionGrants,
  };
}

type AppContextValue = ReturnType<typeof useCinderDockState>;

const AppContext = createContext<AppContextValue | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const value = useCinderDockState();
  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useApp(): AppContextValue {
  const context = useContext(AppContext);
  if (!context) throw new Error('useApp must be used inside AppProvider');
  return context;
}
