import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { ApprovalDialog } from '../components/ApprovalDialog';
import type { ApprovalRequest } from '../lib/types';

const mocks = vi.hoisted(() => ({
  approval: {
    id: 'approval-1',
    conversationId: 'conv-1',
    toolName: 'write_file',
    title: 'Apply file change',
    details: 'Write src/app.ts',
    risk: 'high',
    target: 'src/app.ts',
    diff: '--- a/src/app.ts\n+++ b/src/app.ts\n@@ -1 +1 @@\n-old\n+new',
    allowSession: false,
    sessionKey: 'session:project:write_file',
    createdAt: new Date().toISOString(),
  } as ApprovalRequest,
  respondApproval: vi.fn(),
}));

vi.mock('../state/AppContext', () => ({
  useApp: () => ({
    approval: mocks.approval,
    t: (key: string) => key,
    d: (value: string) => value,
    respondApproval: mocks.respondApproval,
  }),
}));

afterEach(() => {
  cleanup();
  mocks.respondApproval.mockClear();
  mocks.approval.allowSession = false;
});

describe('ApprovalDialog', () => {
  it('shows a real diff and sends the deny decision', () => {
    render(<ApprovalDialog />);
    expect(screen.getByText('Apply file change')).toBeInTheDocument();
    expect(screen.getByText('new')).toBeInTheDocument();
    fireEvent.click(screen.getByText('deny'));
    expect(mocks.respondApproval).toHaveBeenCalledWith('deny', 'approval-1');
  });

  it('offers session approval only when allowed', () => {
    mocks.approval.allowSession = true;
    render(<ApprovalDialog />);
    expect(screen.getByText('approveSession')).toBeInTheDocument();
    fireEvent.click(screen.getByText('approveOnce'));
    expect(mocks.respondApproval).toHaveBeenCalledWith('once', 'approval-1');
  });
});
