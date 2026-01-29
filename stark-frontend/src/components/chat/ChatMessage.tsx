import clsx from 'clsx';
import type { MessageRole } from '@/types';

interface ChatMessageProps {
  role: MessageRole;
  content: string;
  timestamp?: Date;
}

function parseMarkdown(text: string): string {
  // Escape HTML first to prevent XSS
  let parsed = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  // Bold: **text**
  parsed = parsed.replace(/\*\*(.*?)\*\*/g, '<strong class="font-semibold">$1</strong>');

  // Italic: *text* (but not inside **)
  parsed = parsed.replace(/(?<!\*)\*([^*]+)\*(?!\*)/g, '<em>$1</em>');

  // Code blocks: ```code```
  parsed = parsed.replace(/```([\s\S]*?)```/g, '<pre class="bg-slate-900 p-3 rounded-lg my-2 overflow-x-auto text-sm font-mono">$1</pre>');

  // Inline code: `text`
  parsed = parsed.replace(/`([^`]+)`/g, '<code class="bg-slate-700 px-1.5 py-0.5 rounded text-cyan-300 text-sm font-mono">$1</code>');

  // Headers: ## text
  parsed = parsed.replace(/^### (.+)$/gm, '<h3 class="text-lg font-semibold text-white mt-4 mb-2">$1</h3>');
  parsed = parsed.replace(/^## (.+)$/gm, '<h2 class="text-xl font-bold text-white mt-4 mb-2">$1</h2>');

  // Bullet points: - text or • text
  parsed = parsed.replace(/^[-•] (.+)$/gm, '<li class="ml-4 list-disc">$1</li>');

  // Wrap consecutive <li> in <ul>
  parsed = parsed.replace(/(<li[^>]*>.*?<\/li>\n?)+/g, '<ul class="space-y-1 my-2">$&</ul>');

  // Line breaks
  parsed = parsed.replace(/\n/g, '<br/>');

  // Clean up excessive <br/> after block elements
  parsed = parsed.replace(/(<\/h[23]>)<br\/>/g, '$1');
  parsed = parsed.replace(/(<\/ul>)<br\/>/g, '$1');
  parsed = parsed.replace(/(<\/pre>)<br\/>/g, '$1');

  return parsed;
}

export default function ChatMessage({ role, content, timestamp }: ChatMessageProps) {
  const isUser = role === 'user' || role === 'command';
  const isToolIndicator = role === 'tool-indicator';

  const roleStyles: Record<MessageRole, string> = {
    user: 'bg-stark-500 text-white',
    assistant: 'bg-slate-800 text-slate-100',
    system: 'bg-slate-800/50 text-slate-300 border border-slate-700',
    error: 'bg-red-500/20 text-red-400 border border-red-500/50',
    command: 'bg-slate-700 text-stark-400',
    'tool-indicator': 'bg-amber-500/20 text-amber-400 border border-amber-500/50',
    tool: 'bg-purple-500/20 text-purple-300 border border-purple-500/50',
  };

  if (isToolIndicator) {
    return (
      <div className="flex justify-start mb-2">
        <div
          className={clsx(
            'inline-flex items-center gap-2 px-3 py-1.5 rounded-full text-sm',
            roleStyles[role]
          )}
        >
          <span className="w-2 h-2 bg-amber-400 rounded-full animate-pulse" />
          <span>{content}</span>
        </div>
      </div>
    );
  }

  return (
    <div
      className={clsx(
        'flex mb-4',
        isUser ? 'justify-end' : 'justify-start'
      )}
    >
      <div
        className={clsx(
          'max-w-[80%] px-4 py-3 rounded-2xl',
          roleStyles[role],
          isUser ? 'rounded-br-md' : 'rounded-bl-md'
        )}
      >
        {role === 'assistant' || role === 'system' || role === 'tool' ? (
          <div
            className="prose prose-sm prose-invert max-w-none leading-relaxed"
            dangerouslySetInnerHTML={{ __html: parseMarkdown(content) }}
          />
        ) : (
          <p className="whitespace-pre-wrap break-words">{content}</p>
        )}
        {timestamp && (
          <p
            className={clsx(
              'text-xs mt-2',
              isUser ? 'text-white/60' : 'text-slate-500'
            )}
          >
            {timestamp.toLocaleTimeString()}
          </p>
        )}
      </div>
    </div>
  );
}
