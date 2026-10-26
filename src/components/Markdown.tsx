import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { useApp } from '../state/AppContext';

export function Markdown({ content }: { content: string }) {
  const { d } = useApp();
  return (
    <div className="cd-markdown" dir="auto">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ href, children }) => (
            <a
              href={href?.startsWith('http://') || href?.startsWith('https://') ? href : '#'}
              target="_blank"
              rel="noreferrer noopener"
            >
              {children}
            </a>
          ),
          code: ({ children, className }) => (
            <code className={className} dir="auto">
              {children}
            </code>
          ),
          pre: ({ children }) => <pre dir="ltr">{children}</pre>,
        }}
      >
        {d(content)}
      </ReactMarkdown>
    </div>
  );
}
