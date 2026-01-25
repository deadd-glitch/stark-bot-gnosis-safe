import { useState, useEffect } from 'react';
import { Wrench, Check, X } from 'lucide-react';
import Card, { CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { getTools } from '@/lib/api';

interface Tool {
  name: string;
  description: string;
  group: string;
  enabled: boolean;
}

export default function Tools() {
  const [tools, setTools] = useState<Tool[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadTools();
  }, []);

  const loadTools = async () => {
    try {
      const data = await getTools();
      setTools(data);
    } catch (err) {
      setError('Failed to load tools');
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading tools...</span>
        </div>
      </div>
    );
  }

  // Group tools by their group
  const toolsByGroup = tools.reduce((acc, tool) => {
    const group = tool.group || 'other';
    if (!acc[group]) {
      acc[group] = [];
    }
    acc[group].push(tool);
    return acc;
  }, {} as Record<string, Tool[]>);

  const groupLabels: Record<string, string> = {
    web: 'Web Tools',
    filesystem: 'Filesystem Tools',
    exec: 'Execution Tools',
    messaging: 'Messaging Tools',
    system: 'System Tools',
    other: 'Other Tools',
  };

  const groupOrder = ['web', 'filesystem', 'exec', 'messaging', 'system', 'other'];

  return (
    <div className="p-8">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-white mb-2">Tools</h1>
        <p className="text-slate-400">Available tools for your agent</p>
      </div>

      {error && (
        <div className="mb-6 bg-red-500/20 border border-red-500/50 text-red-400 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      <div className="space-y-6">
        {groupOrder.map((groupKey) => {
          const groupTools = toolsByGroup[groupKey];
          if (!groupTools || groupTools.length === 0) return null;

          return (
            <Card key={groupKey}>
              <CardHeader>
                <CardTitle>{groupLabels[groupKey] || groupKey}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  {groupTools.map((tool) => (
                    <div
                      key={tool.name}
                      className="flex items-center justify-between p-4 rounded-lg bg-slate-700/50"
                    >
                      <div className="flex items-center gap-3">
                        <div className="p-2 bg-slate-600 rounded-lg">
                          <Wrench className="w-5 h-5 text-slate-300" />
                        </div>
                        <div>
                          <p className="font-medium text-white">{tool.name}</p>
                          {tool.description && (
                            <p className="text-sm text-slate-400">{tool.description}</p>
                          )}
                        </div>
                      </div>
                      <div
                        className={`p-2 rounded-lg ${
                          tool.enabled
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-slate-600 text-slate-400'
                        }`}
                      >
                        {tool.enabled ? (
                          <Check className="w-5 h-5" />
                        ) : (
                          <X className="w-5 h-5" />
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          );
        })}

        {tools.length === 0 && (
          <Card>
            <CardContent className="text-center py-12">
              <Wrench className="w-12 h-12 text-slate-600 mx-auto mb-4" />
              <p className="text-slate-400">No tools available</p>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
