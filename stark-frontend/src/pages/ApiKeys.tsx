import { useState, useEffect, FormEvent } from 'react';
import { Key, Trash2, Plus, ExternalLink } from 'lucide-react';
import Card, { CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import Input from '@/components/ui/Input';
import { getApiKeys, upsertApiKey, deleteApiKey, ApiKey } from '@/lib/api';

const SERVICES = [
  { value: 'brave_search', label: 'Brave Search' },
  { value: 'serpapi', label: 'SerpAPI' },
  { value: 'github', label: 'GitHub' },
];

const SERVICE_INFO = {
  brave_search: {
    description: 'Get a free API key',
    url: 'https://brave.com/search/api/',
  },
  serpapi: {
    description: 'Get an API key',
    url: 'https://serpapi.com/',
  },
  github: {
    description: 'Create a Personal Access Token with repo scope',
    url: 'https://github.com/settings/tokens',
  },
};

export default function ApiKeys() {
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [selectedService, setSelectedService] = useState('brave_search');
  const [apiKeyInput, setApiKeyInput] = useState('');
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  useEffect(() => {
    loadKeys();
  }, []);

  const loadKeys = async () => {
    try {
      const data = await getApiKeys();
      setKeys(data);
    } catch (err) {
      setMessage({ type: 'error', text: 'Failed to load API keys' });
    } finally {
      setIsLoading(false);
    }
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!apiKeyInput.trim()) {
      setMessage({ type: 'error', text: 'API key cannot be empty' });
      return;
    }

    setIsSaving(true);
    setMessage(null);

    try {
      await upsertApiKey(selectedService, apiKeyInput);
      setMessage({ type: 'success', text: 'API key saved successfully' });
      setApiKeyInput('');
      await loadKeys();
    } catch (err) {
      setMessage({ type: 'error', text: 'Failed to save API key' });
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = async (serviceName: string) => {
    if (!confirm(`Delete API key for ${serviceName}?`)) return;

    try {
      await deleteApiKey(serviceName);
      setMessage({ type: 'success', text: 'API key deleted' });
      await loadKeys();
    } catch (err) {
      setMessage({ type: 'error', text: 'Failed to delete API key' });
    }
  };

  const getServiceLabel = (value: string) => {
    return SERVICES.find((s) => s.value === value)?.label || value;
  };

  if (isLoading) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading API keys...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-white mb-2">API Keys</h1>
        <p className="text-slate-400">
          Manage API keys for external services like web search and GitHub.
        </p>
      </div>

      {message && (
        <div
          className={`mb-6 px-4 py-3 rounded-lg ${
            message.type === 'success'
              ? 'bg-green-500/20 border border-green-500/50 text-green-400'
              : 'bg-red-500/20 border border-red-500/50 text-red-400'
          }`}
        >
          {message.text}
        </div>
      )}

      <div className="grid gap-6 max-w-2xl">
        {/* Add Key Form */}
        <Card>
          <CardHeader>
            <CardTitle>Add API Key</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleSubmit} className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Service
                </label>
                <select
                  value={selectedService}
                  onChange={(e) => setSelectedService(e.target.value)}
                  className="w-full px-4 py-3 bg-slate-900/50 border border-slate-600 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-stark-500 focus:border-transparent"
                >
                  {SERVICES.map((service) => (
                    <option key={service.value} value={service.value}>
                      {service.label}
                    </option>
                  ))}
                </select>
              </div>

              <Input
                type="password"
                label="API Key"
                value={apiKeyInput}
                onChange={(e) => setApiKeyInput(e.target.value)}
                placeholder="Enter your API key"
              />

              <Button type="submit" isLoading={isSaving}>
                <Plus className="w-4 h-4 mr-2" />
                Save Key
              </Button>
            </form>
          </CardContent>
        </Card>

        {/* Service Info */}
        <Card className="border-stark-500/30 bg-stark-500/5">
          <CardContent className="pt-6">
            <div className="flex items-start gap-4">
              <Key className="w-6 h-6 text-stark-400 flex-shrink-0" />
              <div>
                <h4 className="font-medium text-white mb-3">Where to get API keys</h4>
                <ul className="space-y-2 text-sm text-slate-400">
                  {SERVICES.map((service) => {
                    const info = SERVICE_INFO[service.value as keyof typeof SERVICE_INFO];
                    return (
                      <li key={service.value} className="flex items-center gap-2">
                        <span className="text-slate-300 font-medium">{service.label}:</span>
                        <span>{info.description}</span>
                        <a
                          href={info.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-stark-400 hover:text-stark-300 inline-flex items-center gap-1"
                        >
                          <ExternalLink className="w-3 h-3" />
                        </a>
                      </li>
                    );
                  })}
                </ul>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Configured Keys */}
        <Card>
          <CardHeader>
            <CardTitle>Configured Keys</CardTitle>
          </CardHeader>
          <CardContent>
            {keys.length === 0 ? (
              <div className="text-center py-8 text-slate-500">
                <Key className="w-12 h-12 mx-auto mb-3 opacity-50" />
                <p>No API keys configured yet.</p>
                <p className="text-sm mt-1">Add a key above to get started.</p>
              </div>
            ) : (
              <div className="space-y-3">
                {keys.map((key) => (
                  <div
                    key={key.service_name}
                    className="flex items-center justify-between p-4 bg-slate-900/50 rounded-lg border border-slate-700"
                  >
                    <div>
                      <p className="font-medium text-white">
                        {getServiceLabel(key.service_name)}
                      </p>
                      <p className="text-sm text-slate-400 font-mono">{key.key_preview}</p>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDelete(key.service_name)}
                      className="text-red-400 hover:text-red-300 hover:bg-red-500/10"
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
