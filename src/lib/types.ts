export type ProviderKind = 'openAiCompatible' | 'ollamaNative';
export type AuthMethod = 'bearer' | 'none' | 'customHeader';
export type TextMode = 'native' | 'finglish' | 'latin';
export type ApprovalDecisionValue = 'once' | 'session' | 'deny';
export type ModalType = 'providers' | 'models' | 'settings' | 'search' | 'shortcuts' | null;
export type InspectorTab = 'activity' | 'files' | 'terminal';

export interface AuthConfig {
  method: AuthMethod;
  headerName?: string | null;
}

export interface ProviderConfig {
  id: string;
  displayName: string;
  preset: string;
  kind: ProviderKind;
  baseUrl: string;
  auth: AuthConfig;
  extraHeaders: Record<string, string>;
  modelsPath: string;
  chatPath: string;
}

export interface ProviderInput {
  id?: string | null;
  displayName: string;
  preset: string;
  kind: ProviderKind;
  baseUrl: string;
  authMethod: AuthMethod;
  authHeader?: string | null;
  apiKey?: string | null;
  extraHeaders: Record<string, string>;
  modelsPath?: string | null;
  chatPath?: string | null;
}

export interface ProviderView {
  id: string;
  displayName: string;
  preset: string;
  kind: ProviderKind;
  baseUrl: string;
  authMethod: AuthMethod;
  authHeader: string | null;
  hasSecret: boolean;
  extraHeaders: Record<string, string>;
  modelsPath: string;
  chatPath: string;
  modelCount: number;
}

export interface SuggestedModel {
  id: string;
  displayName: string;
  supportsTools: boolean;
  supportsVision: boolean;
  contextWindow?: number | null;
  note: string;
}

export interface ProviderPreset {
  id: string;
  label: string;
  description: string;
  kind: ProviderKind;
  baseUrl: string;
  authMethod: AuthMethod;
  modelsPath: string;
  chatPath: string;
  suggestedModels: SuggestedModel[];
  supportsModelDiscovery: boolean;
  notes: string;
}

export interface RemoteModel {
  id: string;
  displayName: string;
  supportsTools?: boolean | null;
  supportsVision?: boolean | null;
  contextWindow?: number | null;
}

export interface TestConnectionResult {
  ok: boolean;
  status: string;
  message: string;
  modelCount?: number | null;
  latencyMs: number;
}

export interface ModelConfig {
  id: string;
  providerId: string;
  modelId: string;
  displayName: string;
  supportsTools: boolean;
  supportsVision: boolean;
  contextWindow?: number | null;
}

export interface ModelView extends ModelConfig {
  providerName: string;
  isDefault: boolean;
}

export interface ModelInput {
  id?: string | null;
  providerId: string;
  modelId: string;
  displayName: string;
  supportsTools: boolean;
  supportsVision: boolean;
  contextWindow?: number | null;
  setDefault?: boolean;
}

export interface Preferences {
  theme: string;
  uiLanguage: 'fa' | 'en';
  textMode: TextMode;
  fontScale: number;
  desktopToolsEnabled: boolean;
  webAccessEnabled: boolean;
  maxToolIterations: number;
  maxRetries: number;
  terminalVisible: boolean;
  defaultModelId: string | null;
}

export interface UpdatePolicy {
  enabled: boolean;
  reason: string;
}

export interface AppInfo {
  version: string;
  platform: string;
  configDir: string;
  dataDir: string;
  secretBackend: string;
  updateCheckEnabled: boolean;
}

export interface SettingsView {
  preferences: Preferences;
  providers: ProviderView[];
  models: ModelView[];
  projectPath: string | null;
  recentProjects: string[];
  updates: UpdatePolicy;
  app: AppInfo;
}

export interface Bootstrap {
  settings: SettingsView;
  conversations: ConversationSummary[];
  presets: ProviderPreset[];
}

export interface ConversationSummary {
  id: string;
  title: string;
  projectPath: string | null;
  modelId: string | null;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
  preview: string;
}

export interface AttachmentInput {
  path: string;
}

export interface AttachmentView {
  path: string;
  name: string;
  kind: string;
  mime: string;
  size: number;
  text?: string | null;
  dataUrl?: string | null;
}

export interface ToolCallView {
  id: string;
  name: string;
  arguments: string;
}

export interface MessageView {
  id: string;
  conversationId: string;
  role: 'user' | 'assistant' | 'tool' | string;
  content: string;
  attachments: AttachmentView[];
  toolCalls: ToolCallView[];
  toolCallId?: string | null;
  status: string;
  createdAt: string;
  seq: number;
}

export interface ToolEventView {
  id: string;
  conversationId: string;
  kind: string;
  name: string;
  status: string;
  summary: string;
  details: string;
  createdAt: string;
}

export interface FileChangeView {
  id: string;
  conversationId: string;
  path: string;
  before: string;
  after: string;
  diff: string;
  existedBefore: boolean;
  afterHash: string;
  status: string;
  createdAt: string;
}

export interface ApprovalRequest {
  id: string;
  conversationId: string;
  toolName: string;
  title: string;
  details: string;
  risk: string;
  target: string;
  diff?: string | null;
  allowSession: boolean;
  sessionKey: string;
  createdAt: string;
}

export interface TerminalLine {
  stream: string;
  line: string;
}

export interface ProjectInfo {
  path: string;
  name: string;
  fileCount: number;
}

export interface ConversationData {
  conversation: ConversationSummary;
  messages: MessageView[];
  toolEvents: ToolEventView[];
  fileChanges: FileChangeView[];
}

export interface SendMessageInput {
  conversationId: string | null;
  content: string;
  attachments: AttachmentInput[];
  modelId: string | null;
}

export interface RetryInput {
  conversationId: string;
  modelId: string | null;
}

export interface RunStart {
  runId: string;
  conversationId: string;
  userMessage: MessageView | null;
  assistantMessageId: string;
}

export type AgentEvent =
  | { type: 'status'; conversationId: string; status: string; message: string }
  | { type: 'messageDelta'; conversationId: string; messageId: string; delta: string }
  | { type: 'messageDone'; message: MessageView }
  | { type: 'toolCall'; event: ToolEventView; toolCall: ToolCallView }
  | { type: 'toolResult'; event: ToolEventView }
  | { type: 'toolEvent'; event: ToolEventView }
  | {
      type: 'approvalRequest';
      request: ApprovalRequest;
    }
  | {
      type: 'approvalResolved';
      requestId: string;
      decision: 'once' | 'session' | 'deny';
      fromSession: boolean;
    }
  | { type: 'fileChange'; change: FileChangeView }
  | { type: 'terminal'; conversationId: string; lines: TerminalLine[] }
  | { type: 'error'; conversationId: string; message: string; retryable: boolean }
  | { type: 'done'; conversationId: string; messageId: string }
  | { type: 'stopped'; conversationId: string; messageId: string }
  | { type: 'connection'; providerId: string; state: string; message: string };

export interface ConnectionState {
  state: string;
  message: string;
  providerId?: string;
}

export interface ToastState {
  id: number;
  message: string;
  kind: 'info' | 'success' | 'error';
}
