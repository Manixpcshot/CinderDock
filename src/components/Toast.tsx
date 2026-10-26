import { AlertCircle, CheckCircle2, Info, X } from 'lucide-react';
import { useApp } from '../state/AppContext';
import { IconButton } from './Button';

export function ToastHost() {
  const { toast, setToast, t } = useApp();
  if (!toast) return null;
  const Icon =
    toast.kind === 'success' ? CheckCircle2 : toast.kind === 'error' ? AlertCircle : Info;
  return (
    <div className="cd-toast-host" aria-live="polite">
      <div className={`cd-toast cd-toast--${toast.kind}`} role="status">
        <Icon size={17} />
        <span dir="auto">{toast.message}</span>
        <IconButton label={t('dismiss')} onClick={() => setToast(null)} size="sm">
          <X size={14} />
        </IconButton>
      </div>
    </div>
  );
}
