import '@fontsource-variable/vazirmatn/index.css';
import './styles/index.css';
import { createRoot } from 'react-dom/client';
import { AppProvider } from './state/AppContext';
import App from './App';

const container = document.getElementById('root');
if (!container) throw new Error('Root element missing');
createRoot(container).render(
  <AppProvider>
    <App />
  </AppProvider>,
);
