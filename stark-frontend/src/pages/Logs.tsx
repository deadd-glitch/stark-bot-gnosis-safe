import { useState, useEffect, useRef } from 'react';
import { ScrollText, Trash2, Wifi, WifiOff } from 'lucide-react';
import Card, { CardContent } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import { useGateway } from '@/hooks/useGateway';
import clsx from 'clsx';

interface LogEntry {
  id: string;
  event: string;
  data: unknown;
  timestamp: Date;
}

const eventColors: Record<string, string> = {
  'channel.started': 'text-green-400 bg-green-500/20',
  'channel.stopped': 'text-yellow-400 bg-yellow-500/20',
  'channel.error': 'text-red-400 bg-red-500/20',
  'channel.message': 'text-blue-400 bg-blue-500/20',
  'agent.response': 'text-emerald-400 bg-emerald-500/20',
  'tool.execution': 'text-purple-400 bg-purple-500/20',
  'tool.result': 'text-violet-400 bg-violet-500/20',
  'skill.invoked': 'text-pink-400 bg-pink-500/20',
  'connected': 'text-green-400 bg-green-500/20',
  'disconnected': 'text-red-400 bg-red-500/20',
  'error': 'text-red-400 bg-red-500/20',
};

export default function Logs() {
  const { connected, gateway } = useGateway();
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filter, setFilter] = useState<string>('all');
  const [autoScroll, setAutoScroll] = useState(true);
  const logContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleEvent = (payload: unknown) => {
      const { event, data } = payload as { event: string; data: unknown };
      const newLog: LogEntry = {
        id: crypto.randomUUID(),
        event,
        data,
        timestamp: new Date(),
      };
      setLogs((prev) => [...prev.slice(-499), newLog]); // Keep last 500
    };

    gateway.on('*', handleEvent);

    return () => {
      gateway.off('*', handleEvent);
    };
  }, [gateway]);

  // Auto-scroll effect
  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const clearLogs = () => {
    setLogs([]);
  };

  const getEventColor = (event: string) => {
    // Check for exact match first
    if (eventColors[event]) {
      return eventColors[event];
    }
    // Check for prefix match
    for (const [key, value] of Object.entries(eventColors)) {
      if (event.startsWith(key.split('.')[0] + '.')) {
        return value;
      }
    }
    return 'text-slate-400 bg-slate-500/20';
  };

  const formatData = (data: unknown): string => {
    if (!data) return '';
    if (typeof data === 'string') return data;
    const obj = data as Record<string, unknown>;
    if (obj.text) return String(obj.text).slice(0, 100);
    if (obj.from) return `from: ${obj.from}`;
    if (obj.tool_name) return `tool: ${obj.tool_name}`;
    if (obj.skill_name) return `skill: ${obj.skill_name}`;
    if (obj.name) return String(obj.name);
    if (obj.error) return `Error: ${String(obj.error).slice(0, 80)}`;
    return JSON.stringify(data).slice(0, 100);
  };

  const filteredLogs = filter === 'all'
    ? logs
    : logs.filter(log => {
        if (filter === 'error') return log.event.includes('error');
        return log.event.startsWith(filter + '.');
      });

  const filters = ['all', 'channel', 'agent', 'tool', 'skill', 'error'];

  return (
    <div className="p-8 h-full flex flex-col">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-white mb-2">Live Logs</h1>
          <p className="text-slate-400">Real-time event stream from the Gateway</p>
        </div>
        <div className="flex items-center gap-4">
          <div className={clsx(
            'flex items-center gap-2 px-3 py-1.5 rounded-full',
            connected ? 'bg-green-500/20' : 'bg-red-500/20'
          )}>
            {connected ? (
              <>
                <Wifi className="w-4 h-4 text-green-400" />
                <span className="text-sm text-green-400">Connected</span>
              </>
            ) : (
              <>
                <WifiOff className="w-4 h-4 text-red-400" />
                <span className="text-sm text-red-400">Disconnected</span>
              </>
            )}
          </div>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setAutoScroll(!autoScroll)}
          >
            Auto-scroll: {autoScroll ? 'ON' : 'OFF'}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={clearLogs}
          >
            <Trash2 className="w-4 h-4 mr-2" />
            Clear
          </Button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-2 mb-4">
        <span className="text-sm text-slate-400">Filter:</span>
        {filters.map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={clsx(
              'px-3 py-1 rounded-full text-sm transition-colors',
              filter === f
                ? 'bg-stark-500 text-white'
                : 'bg-slate-700 text-slate-300 hover:bg-slate-600'
            )}
          >
            {f.charAt(0).toUpperCase() + f.slice(1)}
          </button>
        ))}
      </div>

      <Card className="flex-1 overflow-hidden">
        <CardContent className="p-0 h-full">
          {filteredLogs.length > 0 ? (
            <div
              ref={logContainerRef}
              className="divide-y divide-slate-700/50 max-h-[60vh] overflow-y-auto font-mono text-sm"
            >
              {filteredLogs.map((log) => (
                <div
                  key={log.id}
                  className="p-3 hover:bg-slate-700/30 transition-colors flex items-start gap-3"
                >
                  <span className="text-slate-500 text-xs whitespace-nowrap">
                    {log.timestamp.toLocaleTimeString('en-US', {
                      hour12: false,
                      hour: '2-digit',
                      minute: '2-digit',
                      second: '2-digit',
                    })}
                  </span>
                  <span
                    className={clsx(
                      'px-2 py-0.5 text-xs font-medium rounded whitespace-nowrap',
                      getEventColor(log.event)
                    )}
                  >
                    {log.event}
                  </span>
                  <span className="text-slate-400 truncate flex-1">
                    {formatData(log.data)}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-64">
              <ScrollText className="w-12 h-12 text-slate-600 mb-4" />
              <p className="text-slate-400">
                {connected
                  ? 'Waiting for events... Events will appear here in real-time.'
                  : 'Not connected to Gateway. Events will appear when connected.'
                }
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Stats Footer */}
      <div className="mt-4 flex items-center justify-between text-sm text-slate-400">
        <div className="flex items-center gap-6">
          <span>Total: <span className="text-white font-medium">{logs.length}</span></span>
          <span>Channels: <span className="text-blue-400 font-medium">{logs.filter(l => l.event.startsWith('channel.')).length}</span></span>
          <span>Agent: <span className="text-emerald-400 font-medium">{logs.filter(l => l.event.startsWith('agent.')).length}</span></span>
          <span>Tools: <span className="text-purple-400 font-medium">{logs.filter(l => l.event.startsWith('tool.')).length}</span></span>
          <span>Errors: <span className="text-red-400 font-medium">{logs.filter(l => l.event.includes('error')).length}</span></span>
        </div>
        {logs.length > 0 && (
          <span className="text-slate-500">
            Last event: {logs[logs.length - 1]?.timestamp.toLocaleTimeString()}
          </span>
        )}
      </div>
    </div>
  );
}
