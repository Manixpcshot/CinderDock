import { listen } from '@tauri-apps/api/event';
import type { AgentEvent } from './types';

export type InvokeFunction = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

type EventAdapter = (handler: (payload: AgentEvent) => void) => () => void;

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke?: InvokeFunction;
    };
    __CINDERDOCK_TEST_INVOKE__?: InvokeFunction;
    __CINDERDOCK_EVENT_ADAPTER__?: EventAdapter;
  }
}

export const AGENT_EVENT_NAME = 'cinderdock-event';

export function backendAvailable(): boolean {
  return Boolean(window.__TAURI_INTERNALS__?.invoke || window.__CINDERDOCK_TEST_INVOKE__);
}

export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const invoke =
    window.__TAURI_INTERNALS__?.invoke ?? window.__CINDERDOCK_TEST_INVOKE__;
  if (!invoke) {
    throw new Error('Rust backend is unavailable. Run CinderDock through Tauri.');
  }
  return (await invoke(command, args ?? {})) as T;
}

export function subscribeAgentEvents(
  handler: (payload: AgentEvent) => void,
): () => void {
  const adapter = window.__CINDERDOCK_EVENT_ADAPTER__;
  if (adapter) return adapter(handler);
  let disposed = false;
  let unlisten: (() => void) | undefined;
  listen<AgentEvent>(AGENT_EVENT_NAME, (event) => handler(event.payload)).then(
    (fn) => {
      if (disposed) fn();
      else unlisten = fn;
    },
    (error) => {
      // The app shows a disconnected state; do not log provider payloads.
      console.warn('CinderDock event listener unavailable', error);
    },
  );
  return () => {
    disposed = true;
    unlisten?.();
  };
}
