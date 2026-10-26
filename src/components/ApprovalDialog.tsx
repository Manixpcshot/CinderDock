import { AlertTriangle, Eye, ShieldAlert, TerminalSquare } from 'lucide-react';
import { useApp } from '../state/AppContext';
import { Badge } from './Badge';
import { Button } from './Button';
import { DiffView } from './DiffView';
import { Modal } from './Modal';

export function ApprovalDialog() {
  const { approval, t, d, respondApproval } = useApp();
  if (!approval) return null;
  const critical = approval.risk === 'critical' || approval.risk === 'high';
  const isDesktop = approval.toolName.startsWith('desktop_');

  return (
    <Modal
      title={t('approvalTitle')}
      subtitle={d(approval.title)}
      onClose={() => undefined}
      dismissible={false}
      size="lg"
      footer={
        <>
          <Button
            variant="danger"
            icon={<ShieldAlert size={15} />}
            onClick={() => void respondApproval('deny', approval.id)}
          >
            {t('deny')}
          </Button>
          <div className="cd-modal__footer-spacer" />
          {approval.allowSession ? (
            <Button
              variant="secondary"
              onClick={() => void respondApproval('session', approval.id)}
              title={t('sessionGrantHint')}
            >
              {t('approveSession')}
            </Button>
          ) : null}
          <Button
            variant="primary"
            onClick={() => void respondApproval('once', approval.id)}
          >
            {t('approveOnce')}
          </Button>
        </>
      }
    >
      <div className="cd-approval" aria-live="assertive">
        <div className="cd-approval__meta">
          <Badge tone={critical ? 'danger' : 'warning'}>
            <AlertTriangle size={13} />
            {critical ? t('criticalRisk') : approval.risk}
          </Badge>
          <Badge tone="muted">{approval.toolName}</Badge>
          <Badge tone={approval.allowSession ? 'accent' : 'muted'}>
            {isDesktop ? t('sessionOnlyOnce') : approval.allowSession ? t('approvalScope') : t('approveOnce')}
          </Badge>
        </div>
        {isDesktop ? (
          <div className="cd-approval__notice">
            <Eye size={16} />
            <span>{t('desktopToolsHint')}</span>
          </div>
        ) : null}
        <section>
          <h3>{t('details')}</h3>
          <pre className="cd-approval__details" dir="auto">
            {d(approval.details)}
          </pre>
        </section>
        <section>
          <h3>
            <TerminalSquare size={14} />
            {t('fileName')} / {t('commandOutput')}
          </h3>
          <code className="cd-approval__target" dir="ltr">
            {approval.target}
          </code>
        </section>
        {approval.diff ? (
          <section>
            <h3>{t('diff')}</h3>
            <DiffView diff={approval.diff} />
          </section>
        ) : null}
        {approval.allowSession ? <p className="cd-approval__hint">{t('sessionGrantHint')}</p> : null}
      </div>
    </Modal>
  );
}
