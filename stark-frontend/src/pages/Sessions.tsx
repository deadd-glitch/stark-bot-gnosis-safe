import { useState, useEffect } from 'react';
import { Calendar, Trash2, MessageSquare, Download, ChevronLeft, User, Bot, Wrench, CheckCircle } from 'lucide-react';
import Card, { CardContent } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import { getSessions, deleteSession, getSessionTranscript, SessionMessage } from '@/lib/api';

interface Session {
  id: number;
  channel_type: string;
  channel_id: number;
  created_at: string;
  updated_at: string;
  message_count?: number;
  completion_status?: 'active' | 'complete';
}

export default function Sessions() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selectedSession, setSelectedSession] = useState<Session | null>(null);
  const [messages, setMessages] = useState<SessionMessage[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingMessages, setIsLoadingMessages] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  useEffect(() => {
    loadSessions();
  }, []);

  const loadSessions = async () => {
    try {
      const data = await getSessions();
      // Sort by updated_at desc and limit to 100
      const sorted = data
        .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
        .slice(0, 100);
      setSessions(sorted);
    } catch (err) {
      setError('Failed to load sessions');
    } finally {
      setIsLoading(false);
    }
  };

  const loadTranscript = async (session: Session) => {
    setSelectedSession(session);
    setIsLoadingMessages(true);
    setError(null);
    try {
      const transcript = await getSessionTranscript(session.id);
      setMessages(transcript.messages);
    } catch (err) {
      setError('Failed to load transcript');
      setMessages([]);
    } finally {
      setIsLoadingMessages(false);
    }
  };

  const handleDelete = async (id: number, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent triggering the card click

    const confirmed = confirm(
      'Force Delete Session?\n\n' +
      'This will:\n' +
      '• Delete the session and all messages\n' +
      '• Cancel any running AI agents/tasks for this session\n' +
      '• Stop any cron jobs using this session\n\n' +
      'This action cannot be undone.'
    );
    if (!confirmed) return;

    setError(null);
    setSuccessMessage(null);

    try {
      const result = await deleteSession(String(id));
      setSessions((prev) => prev.filter((s) => s.id !== id));

      // Show success message with cancelled agents count
      if (result.cancelled_agents && result.cancelled_agents > 0) {
        setSuccessMessage(`Session deleted. Cancelled ${result.cancelled_agents} running agent(s).`);
      } else {
        setSuccessMessage('Session deleted successfully.');
      }

      // Auto-hide success message after 5 seconds
      setTimeout(() => setSuccessMessage(null), 5000);
    } catch (err) {
      setError('Failed to delete session');
    }
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleString();
  };

  const formatShortDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  const exportAsMarkdown = () => {
    if (!selectedSession || messages.length === 0) return;

    let md = `# Chat Session - ${selectedSession.channel_type} (Session ${selectedSession.id})\n\n`;
    md += `**Created:** ${formatDate(selectedSession.created_at)}\n`;
    md += `**Last Updated:** ${formatDate(selectedSession.updated_at)}\n\n`;
    md += `---\n\n`;

    messages.forEach((msg) => {
      const roleEmoji = msg.role === 'user' ? '**User**' : '**Assistant**';
      md += `### ${roleEmoji}\n`;
      md += `*${formatShortDate(msg.created_at)}*\n\n`;
      md += `${msg.content}\n\n`;
      md += `---\n\n`;
    });

    downloadFile(md, `chat-session-${selectedSession.id}.md`, 'text/markdown');
  };

  const exportAsText = () => {
    if (!selectedSession || messages.length === 0) return;

    let txt = `Chat Session - ${selectedSession.channel_type} (Session ${selectedSession.id})\n`;
    txt += `${'='.repeat(60)}\n\n`;
    txt += `Created: ${formatDate(selectedSession.created_at)}\n`;
    txt += `Last Updated: ${formatDate(selectedSession.updated_at)}\n\n`;
    txt += `${'-'.repeat(60)}\n\n`;

    messages.forEach((msg) => {
      const role = msg.role === 'user' ? 'USER' : 'ASSISTANT';
      txt += `[${role}] ${formatShortDate(msg.created_at)}\n`;
      txt += `${msg.content}\n\n`;
      txt += `${'-'.repeat(60)}\n\n`;
    });

    downloadFile(txt, `chat-session-${selectedSession.id}.txt`, 'text/plain');
  };

  const downloadFile = (content: string, filename: string, mimeType: string) => {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  if (isLoading) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading sessions...</span>
        </div>
      </div>
    );
  }

  // Session detail view
  if (selectedSession) {
    return (
      <div className="p-8">
        <div className="mb-6">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setSelectedSession(null)}
            className="mb-4"
          >
            <ChevronLeft className="w-4 h-4 mr-1" />
            Back to sessions
          </Button>
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-white mb-1">
                {selectedSession.channel_type} - Session {selectedSession.id}
              </h1>
              <p className="text-slate-400 text-sm">
                {formatDate(selectedSession.created_at)} - {messages.length} messages
              </p>
            </div>
            <div className="flex gap-2">
              <Button
                variant="secondary"
                size="sm"
                onClick={exportAsMarkdown}
                disabled={messages.length === 0}
              >
                <Download className="w-4 h-4 mr-1" />
                Export MD
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={exportAsText}
                disabled={messages.length === 0}
              >
                <Download className="w-4 h-4 mr-1" />
                Export TXT
              </Button>
            </div>
          </div>
        </div>

        {error && (
          <div className="mb-6 bg-red-500/20 border border-red-500/50 text-red-400 px-4 py-3 rounded-lg">
            {error}
          </div>
        )}

        {isLoadingMessages ? (
          <div className="flex items-center justify-center py-12">
            <div className="flex items-center gap-3">
              <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
              <span className="text-slate-400">Loading messages...</span>
            </div>
          </div>
        ) : messages.length > 0 ? (
          <div className="space-y-4">
            {messages.map((msg) => {
              const roleConfig = {
                user: { icon: User, bg: 'bg-blue-500/20', border: 'border-blue-500/30', iconColor: 'text-blue-400', label: 'User' },
                assistant: { icon: Bot, bg: 'bg-stark-500/20', border: 'border-stark-500/30', iconColor: 'text-stark-400', label: 'Assistant' },
                tool_call: { icon: Wrench, bg: 'bg-amber-500/20', border: 'border-amber-500/30', iconColor: 'text-amber-400', label: 'Tool Call' },
                tool_result: { icon: CheckCircle, bg: 'bg-green-500/20', border: 'border-green-500/30', iconColor: 'text-green-400', label: 'Tool Result' },
                system: { icon: Bot, bg: 'bg-slate-500/20', border: 'border-slate-500/30', iconColor: 'text-slate-400', label: 'System' },
              }[msg.role] || { icon: Bot, bg: 'bg-slate-500/20', border: 'border-slate-500/30', iconColor: 'text-slate-400', label: msg.role };

              const IconComponent = roleConfig.icon;

              return (
                <Card key={msg.id} className={roleConfig.border}>
                  <CardContent>
                    <div className="flex gap-3">
                      <div className={`p-2 rounded-lg ${roleConfig.bg}`}>
                        <IconComponent className={`w-5 h-5 ${roleConfig.iconColor}`} />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <span className="font-medium text-white">
                            {roleConfig.label}
                          </span>
                          <span className="text-xs text-slate-500">
                            {formatShortDate(msg.created_at)}
                          </span>
                        </div>
                        <div className="text-slate-300 whitespace-pre-wrap break-words">
                          {msg.content}
                        </div>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        ) : (
          <Card>
            <CardContent className="text-center py-12">
              <MessageSquare className="w-12 h-12 text-slate-600 mx-auto mb-4" />
              <p className="text-slate-400">No messages in this session</p>
            </CardContent>
          </Card>
        )}
      </div>
    );
  }

  // Sessions list view
  return (
    <div className="p-8">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-white mb-2">Chat Sessions</h1>
        <p className="text-slate-400">View conversation history, export transcripts, or delete sessions</p>
      </div>

      {error && (
        <div className="mb-6 bg-red-500/20 border border-red-500/50 text-red-400 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      {successMessage && (
        <div className="mb-6 bg-green-500/20 border border-green-500/50 text-green-400 px-4 py-3 rounded-lg flex items-center gap-2">
          <CheckCircle className="w-5 h-5" />
          {successMessage}
        </div>
      )}

      {sessions.length > 0 ? (
        <div className="space-y-3">
          {sessions.map((session) => (
            <Card
              key={session.id}
              className="cursor-pointer hover:border-stark-500/50 transition-colors"
              onClick={() => loadTranscript(session)}
            >
              <CardContent>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className="p-3 bg-blue-500/20 rounded-lg">
                      <Calendar className="w-6 h-6 text-blue-400" />
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        <h3 className="font-semibold text-white">
                          {session.channel_type}
                        </h3>
                        <span className="text-xs px-2 py-0.5 bg-slate-700 text-slate-400 rounded">
                          #{session.id}
                        </span>
                        <span className="text-xs font-mono px-2 py-0.5 bg-slate-700/50 text-slate-300 rounded">
                          {session.id.toString(16).padStart(8, '0')}
                        </span>
                        {session.completion_status === 'complete' && (
                          <span className="text-xs px-2 py-0.5 bg-green-500/20 text-green-400 rounded-full flex items-center gap-1">
                            <CheckCircle className="w-3 h-3" />
                            Complete
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-4 mt-1 text-sm text-slate-400">
                        <span>Last active: {formatShortDate(session.updated_at)}</span>
                        {session.message_count !== undefined && (
                          <span className="flex items-center gap-1">
                            <MessageSquare className="w-3 h-3" />
                            {session.message_count} messages
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => handleDelete(session.id, e)}
                      className="text-red-400 hover:text-red-300 hover:bg-red-500/20"
                      title="Force delete session and cancel running agents"
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                    <ChevronLeft className="w-5 h-5 text-slate-500 rotate-180" />
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <Card>
          <CardContent className="text-center py-12">
            <Calendar className="w-12 h-12 text-slate-600 mx-auto mb-4" />
            <p className="text-slate-400">No sessions found</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
