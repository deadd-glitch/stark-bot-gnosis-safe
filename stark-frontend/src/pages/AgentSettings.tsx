import { useState, useEffect, FormEvent } from 'react';
import { Save } from 'lucide-react';
import Card, { CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import Input from '@/components/ui/Input';
import { getAgentSettings, updateAgentSettings } from '@/lib/api';

interface Settings {
  provider?: string;
  endpoint?: string;
  api_key?: string;
  model?: string;
}

export default function AgentSettings() {
  const [settings, setSettings] = useState<Settings>({});
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const data = await getAgentSettings();
      setSettings(data as Settings);
    } catch (err) {
      setMessage({ type: 'error', text: 'Failed to load settings' });
    } finally {
      setIsLoading(false);
    }
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setIsSaving(true);
    setMessage(null);

    try {
      await updateAgentSettings({
        provider: settings.provider || 'openai_compatible',
        endpoint: settings.endpoint || '',
        api_key: settings.api_key || '',
        model: settings.model || '',
      });
      setMessage({ type: 'success', text: 'Settings saved successfully' });
      // Reload to confirm saved values
      await loadSettings();
    } catch (err) {
      setMessage({ type: 'error', text: 'Failed to save settings' });
    } finally {
      setIsSaving(false);
    }
  };

  const handleProviderChange = (provider: string) => {
    setSettings((prev) => ({
      ...prev,
      provider,
    }));
  };

  const updateField = (field: keyof Settings, value: string) => {
    setSettings((prev) => ({ ...prev, [field]: value }));
  };

  if (isLoading) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading settings...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-white mb-2">Agent Settings</h1>
        <p className="text-slate-400">Configure your AI agent's provider and connection</p>
      </div>

      <form onSubmit={handleSubmit}>
        <div className="grid gap-6 max-w-2xl">
          <Card>
            <CardHeader>
              <CardTitle>Provider Configuration</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Provider Type
                </label>
                <select
                  value={settings.provider || 'openai_compatible'}
                  onChange={(e) => handleProviderChange(e.target.value)}
                  className="w-full px-4 py-3 bg-slate-900/50 border border-slate-600 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-stark-500 focus:border-transparent"
                >
                  <option value="openai_compatible">OpenAI Compatible (DigitalOcean, Azure, etc.)</option>
                  <option value="openai">OpenAI</option>
                  <option value="claude">Anthropic (Claude)</option>
                  <option value="llama">Ollama (Local)</option>
                </select>
                <p className="text-xs text-slate-500 mt-1">
                  Most cloud AI endpoints use OpenAI-compatible API format
                </p>
              </div>

              <Input
                label="API Endpoint URL"
                value={settings.endpoint || ''}
                onChange={(e) => updateField('endpoint', e.target.value)}
                placeholder="https://your-endpoint.com/v1/chat/completions"
              />

              <Input
                label="Model Name (optional)"
                value={settings.model || ''}
                onChange={(e) => updateField('model', e.target.value)}
                placeholder="gpt-4o, claude-sonnet-4, llama3.2, etc."
              />

              <Input
                label="API Key"
                value={settings.api_key || ''}
                onChange={(e) => updateField('api_key', e.target.value)}
                placeholder="Enter your API key"
              />
            </CardContent>
          </Card>

          {message && (
            <div
              className={`px-4 py-3 rounded-lg ${
                message.type === 'success'
                  ? 'bg-green-500/20 border border-green-500/50 text-green-400'
                  : 'bg-red-500/20 border border-red-500/50 text-red-400'
              }`}
            >
              {message.text}
            </div>
          )}

          <Button type="submit" isLoading={isSaving} className="w-fit">
            <Save className="w-4 h-4 mr-2" />
            Save Settings
          </Button>
        </div>
      </form>
    </div>
  );
}
