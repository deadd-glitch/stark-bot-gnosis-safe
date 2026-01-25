import { useState, useEffect } from 'react';
import { Bug, Wifi, WifiOff, Server } from 'lucide-react';
import Card, { CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import { useGateway } from '@/hooks/useGateway';

interface DebugInfo {
  version?: string;
  uptime?: number;
  memory?: {
    used: number;
    total: number;
  };
  database?: {
    connected: boolean;
    size?: number;
  };
}

export default function Debug() {
  const { connected, gateway, connect, disconnect } = useGateway();
  const [events, setEvents] = useState<Array<{ event: string; data: unknown; time: Date }>>([]);
  const [debugInfo] = useState<DebugInfo>({});

  useEffect(() => {
    const handleEvent = (payload: unknown) => {
      const { event, data } = payload as { event: string; data: unknown };
      setEvents((prev) => [
        { event, data, time: new Date() },
        ...prev.slice(0, 99), // Keep last 100 events
      ]);
    };

    gateway.on('*', handleEvent);

    return () => {
      gateway.off('*', handleEvent);
    };
  }, [gateway]);

  const clearEvents = () => {
    setEvents([]);
  };

  const formatUptime = (seconds?: number) => {
    if (!seconds) return 'N/A';
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (days > 0) return `${days}d ${hours}h ${minutes}m`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
  };

  const formatBytes = (bytes?: number) => {
    if (!bytes) return 'N/A';
    const units = ['B', 'KB', 'MB', 'GB'];
    let value = bytes;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex++;
    }
    return `${value.toFixed(1)} ${units[unitIndex]}`;
  };

  return (
    <div className="p-8">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-white mb-2">Debug</h1>
        <p className="text-slate-400">System diagnostics and debugging tools</p>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* Gateway Status */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              {connected ? (
                <Wifi className="w-5 h-5 text-green-400" />
              ) : (
                <WifiOff className="w-5 h-5 text-red-400" />
              )}
              Gateway Connection
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-center justify-between p-3 rounded-lg bg-slate-700/50">
                <span className="text-slate-300">Status</span>
                <span
                  className={`flex items-center gap-2 ${
                    connected ? 'text-green-400' : 'text-red-400'
                  }`}
                >
                  <span
                    className={`w-2 h-2 rounded-full ${
                      connected ? 'bg-green-400' : 'bg-red-400'
                    }`}
                  />
                  {connected ? 'Connected' : 'Disconnected'}
                </span>
              </div>
              <Button
                variant={connected ? 'danger' : 'primary'}
                onClick={connected ? disconnect : connect}
                className="w-full"
              >
                {connected ? 'Disconnect' : 'Connect'}
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* System Info */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Server className="w-5 h-5 text-blue-400" />
              System Information
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <div className="flex items-center justify-between p-3 rounded-lg bg-slate-700/50">
                <span className="text-slate-300">Version</span>
                <span className="text-white font-mono">
                  {debugInfo.version || '0.1.0'}
                </span>
              </div>
              <div className="flex items-center justify-between p-3 rounded-lg bg-slate-700/50">
                <span className="text-slate-300">Uptime</span>
                <span className="text-white">
                  {formatUptime(debugInfo.uptime)}
                </span>
              </div>
              <div className="flex items-center justify-between p-3 rounded-lg bg-slate-700/50">
                <span className="text-slate-300">Memory</span>
                <span className="text-white">
                  {formatBytes(debugInfo.memory?.used)} /{' '}
                  {formatBytes(debugInfo.memory?.total)}
                </span>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Event Log */}
        <Card className="lg:col-span-2">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <Bug className="w-5 h-5 text-amber-400" />
                Gateway Events
              </CardTitle>
              <Button variant="ghost" size="sm" onClick={clearEvents}>
                Clear
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            {events.length > 0 ? (
              <div className="space-y-2 max-h-96 overflow-y-auto font-mono text-sm">
                {events.map((event, index) => (
                  <div
                    key={index}
                    className="p-3 rounded-lg bg-slate-700/50 hover:bg-slate-700 transition-colors"
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-stark-400">{event.event}</span>
                      <span className="text-xs text-slate-500">
                        {event.time.toLocaleTimeString()}
                      </span>
                    </div>
                    <pre className="text-xs text-slate-400 overflow-x-auto">
                      {JSON.stringify(event.data, null, 2)}
                    </pre>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-8">
                <Bug className="w-8 h-8 text-slate-600 mx-auto mb-2" />
                <p className="text-slate-400 text-sm">
                  No events captured yet. Events will appear here in real-time.
                </p>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
