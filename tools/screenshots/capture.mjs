import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
import chromiumPackage from '@sparticuz/chromium';
import playwright from 'playwright-core';

const { chromium } = playwright;

const root = process.cwd();
const outDir = path.join(root, 'docs', 'screenshots');
const port = 4173;
const baseUrl = `http://127.0.0.1:${port}`;
fs.mkdirSync(outDir, { recursive: true });

const now = new Date().toISOString();
const projectPath = 'D:\\Projects\\CinderDockDemo';

const presets = [
  {
    id: 'openai',
    label: 'OpenAI',
    description: 'سازگار با OpenAI Chat Completions و فهرست /models',
    kind: 'openAiCompatible',
    baseUrl: 'https://api.openai.com/v1',
    authMethod: 'bearer',
    modelsPath: 'models',
    chatPath: 'chat/completions',
    suggestedModels: [],
    supportsModelDiscovery: true,
    notes: 'کلید فقط در مخزن امن ویندوز نگهداری میشود.',
  },
  {
    id: 'ollama-native',
    label: 'Ollama (اتصالدهنده بومی)',
    description: 'از /api/chat و /api/tags بومی Ollama استفاده میکند',
    kind: 'ollamaNative',
    baseUrl: 'http://127.0.0.1:11434',
    authMethod: 'none',
    modelsPath: 'api/tags',
    chatPath: 'api/chat',
    suggestedModels: [],
    supportsModelDiscovery: true,
    notes: 'اتصالدهنده بومی، بدون ادعای سازگاری OpenAI.',
  },
  {
    id: 'lmstudio',
    label: 'LM Studio',
    description: 'سرور محلی سازگار با OpenAI Chat Completions',
    kind: 'openAiCompatible',
    baseUrl: 'http://127.0.0.1:1234/v1',
    authMethod: 'none',
    modelsPath: 'models',
    chatPath: 'chat/completions',
    suggestedModels: [],
    supportsModelDiscovery: true,
    notes: 'سرور محلی را پیش از آزمایش روشن کنید.',
  },
];

let settings = {
  preferences: {
    theme: 'ember-dark',
    uiLanguage: 'fa',
    textMode: 'native',
    fontScale: 1,
    desktopToolsEnabled: false,
    webAccessEnabled: true,
    maxToolIterations: 8,
    maxRetries: 2,
    terminalVisible: false,
    defaultModelId: 'model-gpt',
  },
  providers: [
    {
      id: 'provider-openai',
      displayName: 'OpenAI',
      preset: 'openai',
      kind: 'openAiCompatible',
      baseUrl: 'https://api.openai.com/v1',
      authMethod: 'bearer',
      authHeader: null,
      hasSecret: true,
      extraHeaders: {},
      modelsPath: 'models',
      chatPath: 'chat/completions',
      modelCount: 2,
    },
    {
      id: 'provider-ollama',
      displayName: 'Ollama (local)',
      preset: 'ollama-native',
      kind: 'ollamaNative',
      baseUrl: 'http://127.0.0.1:11434',
      authMethod: 'none',
      authHeader: null,
      hasSecret: false,
      extraHeaders: {},
      modelsPath: 'api/tags',
      chatPath: 'api/chat',
      modelCount: 1,
    },
  ],
  models: [
    {
      id: 'model-gpt',
      providerId: 'provider-openai',
      modelId: 'gpt-4o',
      displayName: 'GPT-4o',
      supportsTools: true,
      supportsVision: true,
      contextWindow: 128000,
      providerName: 'OpenAI',
      isDefault: true,
    },
    {
      id: 'model-mini',
      providerId: 'provider-openai',
      modelId: 'gpt-4o-mini',
      displayName: 'GPT-4o mini',
      supportsTools: true,
      supportsVision: true,
      contextWindow: 128000,
      providerName: 'OpenAI',
      isDefault: false,
    },
    {
      id: 'model-qwen',
      providerId: 'provider-ollama',
      modelId: 'qwen2.5-coder',
      displayName: 'Qwen 2.5 Coder',
      supportsTools: true,
      supportsVision: false,
      contextWindow: 32768,
      providerName: 'Ollama (local)',
      isDefault: false,
    },
  ],
  projectPath,
  recentProjects: [
    projectPath,
    'D:\\Projects\\EmberNotes',
    'D:\\Projects\\LocalTools',
  ],
  updates: {
    enabled: false,
    reason: 'In-app updates stay disabled until release signatures and private-key custody are configured.',
  },
  app: {
    version: '0.1.0',
    platform: 'windows',
    configDir: 'C:\\Users\\Public\\AppData\\Roaming\\CinderDock',
    dataDir: 'C:\\Users\\Public\\AppData\\Roaming\\CinderDock\\data',
    secretBackend: 'Windows DPAPI (per-user)',
    updateCheckEnabled: false,
  },
};

const conversations = [
  {
    id: 'conv-1',
    title: 'رفع خطای خواندن تنظیمات',
    projectPath,
    modelId: 'model-gpt',
    createdAt: '2026-10-25T08:30:00.000Z',
    updatedAt: '2026-10-26T09:42:00.000Z',
    messageCount: 5,
    preview: 'مشکل از مسیر خالی بود؛ یک مقدار پیشفرض امن اضافه شد.',
  },
  {
    id: 'conv-2',
    title: 'ساخت تست برای ابزارها',
    projectPath,
    modelId: 'model-qwen',
    createdAt: '2026-10-24T12:10:00.000Z',
    updatedAt: '2026-10-25T16:04:00.000Z',
    messageCount: 8,
    preview: 'چهار تست جدید برای امنیت مسیرها اضافه شد.',
  },
  {
    id: 'conv-3',
    title: 'بررسی ساختار پروژه',
    projectPath: null,
    modelId: 'model-mini',
    createdAt: '2026-10-21T07:00:00.000Z',
    updatedAt: '2026-10-22T11:20:00.000Z',
    messageCount: 3,
    preview: 'ساختار پوشهها و وابستگیها خلاصه شد.',
  },
];

const diff = [
  '--- a/src/config/loader.ts',
  '+++ b/src/config/loader.ts',
  '@@ -8,7 +8,9 @@',
  ' export function loadConfig(path?: string): Config {',
  '-  const target = path!;',
  '-  const raw = readFileSync(target, "utf8");',
  '+  const target = path ?? DEFAULT_CONFIG_PATH;',
  '+  if (!existsSync(target)) {',
  '+    return { ...DEFAULT_CONFIG, source: "defaults" };',
  '+  }',
  '+  const raw = readFileSync(target, "utf8");',
  '   return { ...DEFAULT_CONFIG, ...JSON.parse(raw), source: target };',
  ' }',
].join('\n');

const conversationData = {
  conversation: conversations[0],
  messages: [
    {
      id: 'msg-1',
      conversationId: 'conv-1',
      role: 'user',
      content: 'لطفاً خطای خواندن تنظیمات را پیدا کن و اصلاحش کن.',
      attachments: [],
      toolCalls: [],
      toolCallId: null,
      status: 'complete',
      createdAt: '2026-10-26T09:38:00.000Z',
      seq: 1,
    },
    {
      id: 'msg-2',
      conversationId: 'conv-1',
      role: 'assistant',
      content: 'فایل `src/config/loader.ts` را بررسی میکنم. به نظر میرسد وقتی مسیری داده نشود، مقدار `undefined` مستقیماً به `readFileSync` میرسد.',
      attachments: [],
      toolCalls: [{ id: 'call-1', name: 'read_file', arguments: '{"path":"src/config/loader.ts"}' }],
      toolCallId: null,
      status: 'complete',
      createdAt: '2026-10-26T09:39:00.000Z',
      seq: 2,
    },
    {
      id: 'msg-3',
      conversationId: 'conv-1',
      role: 'tool',
      content: 'FILE: src/config/loader.ts\n...',
      attachments: [],
      toolCalls: [],
      toolCallId: 'call-1',
      status: 'complete',
      createdAt: '2026-10-26T09:39:20.000Z',
      seq: 3,
    },
    {
      id: 'msg-4',
      conversationId: 'conv-1',
      role: 'assistant',
      content: 'یک مقدار پیشفرض امن و بررسی وجود فایل اضافه کردم. تغییر پیشنهادی در پنل **تغییرات فایلها** قابل بازبینی و بازگردانی است.\n\n```ts\nconst target = path ?? DEFAULT_CONFIG_PATH;\nif (!existsSync(target)) return { ...DEFAULT_CONFIG, source: "defaults" };\n```',
      attachments: [],
      toolCalls: [{ id: 'call-2', name: 'write_file', arguments: '{"path":"src/config/loader.ts","content":"..."}' }],
      toolCallId: null,
      status: 'complete',
      createdAt: '2026-10-26T09:42:00.000Z',
      seq: 4,
    },
  ],
  toolEvents: [
    {
      id: 'event-1',
      conversationId: 'conv-1',
      kind: 'tool_call',
      name: 'read_file',
      status: 'running',
      summary: 'درخواست اجرای read_file',
      details: '{"path":"src/config/loader.ts"}',
      createdAt: '2026-10-26T09:39:00.000Z',
    },
    {
      id: 'event-2',
      conversationId: 'conv-1',
      kind: 'tool_result',
      name: 'read_file',
      status: 'success',
      summary: 'نتیجهٔ واقعی ابزار دریافت شد',
      details: 'FILE: src/config/loader.ts',
      createdAt: '2026-10-26T09:39:20.000Z',
    },
    {
      id: 'event-3',
      conversationId: 'conv-1',
      kind: 'approval',
      name: 'write_file',
      status: 'once',
      summary: 'تصمیم کاربر ثبت شد',
      details: 'اعمال تغییر فایل src/config/loader.ts',
      createdAt: '2026-10-26T09:41:00.000Z',
    },
    {
      id: 'event-4',
      conversationId: 'conv-1',
      kind: 'tool_result',
      name: 'write_file',
      status: 'success',
      summary: 'نتیجهٔ واقعی ابزار دریافت شد',
      details: 'فایل پس از تأیید کاربر نوشته شد.',
      createdAt: '2026-10-26T09:42:00.000Z',
    },
  ],
  fileChanges: [
    {
      id: 'change-1',
      conversationId: 'conv-1',
      path: 'src/config/loader.ts',
      before: 'export function loadConfig(path?: string): Config {\n  const target = path!;\n  const raw = readFileSync(target, "utf8");\n  return { ...DEFAULT_CONFIG, ...JSON.parse(raw), source: target };\n}',
      after: 'export function loadConfig(path?: string): Config {\n  const target = path ?? DEFAULT_CONFIG_PATH;\n  if (!existsSync(target)) {\n    return { ...DEFAULT_CONFIG, source: "defaults" };\n  }\n  const raw = readFileSync(target, "utf8");\n  return { ...DEFAULT_CONFIG, ...JSON.parse(raw), source: target };\n}',
      diff,
      existedBefore: true,
      afterHash: 'preview-only-hash',
      status: 'applied',
      createdAt: '2026-10-26T09:42:00.000Z',
    },
  ],
};

const remoteModels = [
  { id: 'gpt-4o', displayName: 'GPT-4o', supportsTools: true, supportsVision: true, contextWindow: 128000 },
  { id: 'gpt-4o-mini', displayName: 'GPT-4o mini', supportsTools: true, supportsVision: true, contextWindow: 128000 },
  { id: 'o1-preview', displayName: 'o1-preview', supportsTools: null, supportsVision: null, contextWindow: null },
];

const projectFiles = [
  'README.md',
  'package.json',
  'src/App.tsx',
  'src/config/loader.ts',
  'src/lib/types.ts',
  'src/state/AppContext.tsx',
  'src-tauri/src/agent.rs',
  'src-tauri/src/tools.rs',
  'src-tauri/tauri.conf.json',
];

const approvalEvent = {
  type: 'approvalRequest',
  request: {
    id: 'approval-preview',
    conversationId: 'conv-1',
    toolName: 'write_file',
    title: 'اعمال تغییر فایل',
    details: 'مدل میخواهد فایل «src/config/loader.ts» را تغییر دهد. ۵ خط اضافه و ۲ خط حذف میشود. diff زیر پیش از اعمال نشان داده شده است.',
    risk: 'high',
    target: 'src/config/loader.ts',
    diff,
    allowSession: true,
    sessionKey: 'session:project:write_file',
    createdAt: now,
  },
};

const initData = {
  settings,
  conversations,
  presets,
  conversationData,
  remoteModels,
  projectFiles,
  approvalEvent,
};

function startPreview() {
  const child = spawn(
    process.execPath,
    ['node_modules/vite/bin/vite.js', 'preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'],
    { cwd: root, stdio: ['ignore', 'pipe', 'pipe'] },
  );
  child.stdout.on('data', (chunk) => process.stdout.write(`[preview] ${chunk}`));
  child.stderr.on('data', (chunk) => process.stderr.write(`[preview] ${chunk}`));
  return child;
}

async function waitForServer() {
  for (let attempt = 0; attempt < 60; attempt++) {
    try {
      const response = await fetch(baseUrl);
      if (response.ok) return;
    } catch {
      // wait until Vite preview is ready
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error('Vite preview did not start');
}

function prepareChromiumLibs() {
  if (process.platform !== 'linux') return process.env;
  const libRoot = path.join(os.tmpdir(), 'cinderdock-chromium-libs');
  const libDir = path.join(libRoot, 'lib');
  fs.mkdirSync(libRoot, { recursive: true });
  if (!fs.existsSync(path.join(libDir, 'libnspr4.so'))) {
    const compressed = fs.readFileSync(
      path.join(root, 'node_modules', '@sparticuz', 'chromium', 'bin', 'al2023.tar.br'),
    );
    const tarPath = path.join(libRoot, 'al2023.tar');
    fs.writeFileSync(tarPath, zlib.brotliDecompressSync(compressed));
    execFileSync('tar', ['-xf', tarPath, '-C', libRoot]);
  }
  const env = { ...process.env };
  env.LD_LIBRARY_PATH = [libDir, os.tmpdir(), env.LD_LIBRARY_PATH]
    .filter(Boolean)
    .join(path.delimiter);
  return env;
}

function installMocks(data) {
  const handlers = [];
  window.__CINDERDOCK_EVENT_ADAPTER__ = (handler) => {
    handlers.push(handler);
    return () => {
      const index = handlers.indexOf(handler);
      if (index >= 0) handlers.splice(index, 1);
    };
  };
  window.__CINDERDOCK_DISPATCH__ = (event) => {
    for (const handler of [...handlers]) handler(event);
  };

  let mockSettings = data.settings;
  const okConnection = {
    ok: true,
    status: 'online',
    message: 'اتصال برقرار شد و فهرست مدلها دریافت شد.',
    modelCount: 3,
    latencyMs: 184,
  };

  window.__CINDERDOCK_TEST_INVOKE__ = async (command, args = {}) => {
    switch (command) {
      case 'get_bootstrap':
        return { settings: mockSettings, conversations: data.conversations, presets: data.presets };
      case 'get_settings':
        return mockSettings;
      case 'list_provider_presets':
        return data.presets;
      case 'list_conversations':
        return data.conversations;
      case 'load_conversation':
        return data.conversationData;
      case 'search_conversations':
        return data.conversations.filter((item) =>
          `${item.title} ${item.preview}`.toLowerCase().includes(String(args.query ?? '').toLowerCase()),
        );
      case 'create_conversation':
        return {
          id: 'conv-new',
          title: args.title ?? 'گفتوگوی جدید',
          projectPath: mockSettings.projectPath,
          modelId: args.modelId ?? mockSettings.preferences.defaultModelId,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          messageCount: 0,
          preview: '',
        };
      case 'delete_conversation':
        return data.conversations.filter((item) => item.id !== args.conversationId);
      case 'set_conversation_model':
        return data.conversations[0];
      case 'list_project_files':
        return data.projectFiles;
      case 'choose_project_folder':
      case 'set_project':
        return { path: mockSettings.projectPath, name: 'CinderDockDemo', fileCount: data.projectFiles.length };
      case 'choose_attachments':
        return [];
      case 'test_provider':
      case 'test_saved_provider':
        return okConnection;
      case 'save_provider':
        return mockSettings.providers[0];
      case 'delete_provider':
        return mockSettings;
      case 'fetch_provider_models':
        return data.remoteModels;
      case 'save_model':
      case 'save_models':
      case 'delete_model':
      case 'set_default_model':
      case 'update_preferences':
        if (command === 'update_preferences' && args.patch) {
          mockSettings = { ...mockSettings, preferences: { ...mockSettings.preferences, ...args.patch } };
        }
        return mockSettings;
      case 'send_message':
      case 'retry_last':
        return {
          runId: 'run-preview',
          conversationId: args.input?.conversationId ?? 'conv-1',
          userMessage: null,
          assistantMessageId: 'msg-preview',
        };
      case 'stop_generation':
      case 'respond_approval':
      case 'clear_session_grants':
        return true;
      case 'revert_file_change':
        return {
          ...data.conversationData.fileChanges[0],
          status: 'reverted',
        };
      default:
        return null;
    }
  };
}

async function shot(page, name) {
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(180);
  await page.screenshot({ path: path.join(outDir, name), animations: 'disabled' });
  console.log('saved', name);
}

const preview = startPreview();
let browser;

try {
  await waitForServer();
  const executablePath =
    process.env.CHROME_PATH ?? (await chromiumPackage.executablePath('/tmp/cinderdock-chromium'));
  const launchEnv = prepareChromiumLibs();
  browser = await chromium.launch({
    args: chromiumPackage.args,
    executablePath,
    headless: chromiumPackage.headless,
    env: launchEnv,
  });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
    locale: 'fa-IR',
    colorScheme: 'dark',
    reducedMotion: 'reduce',
  });
  await context.addInitScript(installMocks, initData);
  const page = await context.newPage();
  page.on('pageerror', (error) => console.error('[pageerror]', error.message));
  await page.goto(baseUrl, { waitUntil: 'networkidle' });

  await shot(page, '01-launch-rtl.png');

  await page.getByText('رفع خطای خواندن تنظیمات', { exact: false }).first().click();
  await page.waitForTimeout(250);
  await page.evaluate(() => {
    window.__CINDERDOCK_DISPATCH__({
      type: 'terminal',
      conversationId: 'conv-1',
      lines: [
        { stream: 'stdout', line: '$ npm test -- --run' },
        { stream: 'stdout', line: '✓ security path tests (5)' },
        { stream: 'stdout', line: '✓ stream parser tests (3)' },
        { stream: 'stdout', line: 'Test Files  4 passed (4)' },
      ],
    });
  });
  await page.waitForTimeout(150);
  await shot(page, '02-conversation-activity.png');

  await page.locator('[data-testid="tab-files"]').click();
  await page.getByText('نمایش diff', { exact: false }).first().click();
  await shot(page, '03-file-diff.png');

  await page.evaluate((event) => window.__CINDERDOCK_DISPATCH__(event), initData.approvalEvent);
  await page.waitForTimeout(160);
  await shot(page, '04-approval.png');

  await page.getByText('رد کردن', { exact: false }).first().click();
  await page.waitForTimeout(120);
  await page.locator('[data-testid="nav-providers"]').click();
  await shot(page, '05-provider-connection.png');

  await page.getByRole('button', { name: 'Close' }).first().click();
  await page.locator('[data-testid="nav-models"]').click();
  await shot(page, '06-model-manager.png');

  await page.getByRole('button', { name: 'Close' }).first().click();
  await page.locator('[data-testid="nav-settings"]').click();
  await page.getByText('فینگلیش', { exact: false }).first().click();
  await page.getByText('ذخیره', { exact: false }).last().click();
  await page.keyboard.press('Escape');
  await page.waitForTimeout(4800);
  await shot(page, '07-finglish-latin.png');

  await context.close();
} finally {
  if (browser) await browser.close();
  preview.kill('SIGTERM');
}
