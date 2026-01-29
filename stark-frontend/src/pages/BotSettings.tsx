import { useState, useEffect, FormEvent } from 'react';
import { Save, Bot, Shield, AlertCircle } from 'lucide-react';
import Card, { CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import Input from '@/components/ui/Input';
import { getBotSettings, updateBotSettings, BotSettings as BotSettingsType } from '@/lib/api';

export default function BotSettings() {
  const [, setSettings] = useState<BotSettingsType | null>(null);
  const [botName, setBotName] = useState('StarkBot');
  const [botEmail, setBotEmail] = useState('starkbot@users.noreply.github.com');
  const [web3TxRequiresConfirmation, setWeb3TxRequiresConfirmation] = useState(true);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const data = await getBotSettings();
      setSettings(data);
      setBotName(data.bot_name);
      setBotEmail(data.bot_email);
      setWeb3TxRequiresConfirmation(data.web3_tx_requires_confirmation);
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
      const updated = await updateBotSettings({
        bot_name: botName,
        bot_email: botEmail,
        web3_tx_requires_confirmation: web3TxRequiresConfirmation,
      });
      setSettings(updated);
      setMessage({ type: 'success', text: 'Settings saved successfully' });
    } catch (err) {
      setMessage({ type: 'error', text: 'Failed to save settings' });
    } finally {
      setIsSaving(false);
    }
  };

  const toggleConfirmation = async () => {
    setIsSaving(true);
    setMessage(null);

    try {
      const updated = await updateBotSettings({
        web3_tx_requires_confirmation: !web3TxRequiresConfirmation,
      });
      setSettings(updated);
      setWeb3TxRequiresConfirmation(updated.web3_tx_requires_confirmation);
      setMessage({ type: 'success', text: 'Setting updated' });
    } catch (err) {
      setMessage({ type: 'error', text: 'Failed to update setting' });
    } finally {
      setIsSaving(false);
    }
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
        <h1 className="text-2xl font-bold text-white mb-2">Bot Settings</h1>
        <p className="text-slate-400">Configure bot identity and transaction security</p>
      </div>

      <div className="grid gap-6 max-w-2xl">
        {/* Bot Identity Section */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Bot className="w-5 h-5 text-stark-400" />
              Bot Identity
            </CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleSubmit} className="space-y-4">
              <Input
                label="Bot Name"
                value={botName}
                onChange={(e) => setBotName(e.target.value)}
                placeholder="StarkBot"
              />
              <p className="text-xs text-slate-500 -mt-2">
                Used for git commits and identification
              </p>

              <Input
                label="Bot Email"
                value={botEmail}
                onChange={(e) => setBotEmail(e.target.value)}
                placeholder="starkbot@users.noreply.github.com"
                type="email"
              />
              <p className="text-xs text-slate-500 -mt-2">
                Used for git commit author email
              </p>

              <Button type="submit" isLoading={isSaving} className="w-fit">
                <Save className="w-4 h-4 mr-2" />
                Save Identity
              </Button>
            </form>
          </CardContent>
        </Card>

        {/* Transaction Security Section */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <Shield className="w-5 h-5 text-stark-400" />
                Transaction Security
              </CardTitle>
              <button
                onClick={toggleConfirmation}
                disabled={isSaving}
                className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                  web3TxRequiresConfirmation ? 'bg-stark-500' : 'bg-slate-600'
                }`}
              >
                <span
                  className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                    web3TxRequiresConfirmation ? 'translate-x-6' : 'translate-x-1'
                  }`}
                />
              </button>
            </div>
          </CardHeader>
          <CardContent>
            <div className="bg-slate-800/50 rounded-lg p-4">
              <div className="flex items-start gap-3">
                <AlertCircle className="w-5 h-5 text-stark-400 mt-0.5" />
                <div>
                  <p className="text-sm font-medium text-white mb-1">
                    Require confirmation for Web3 transactions
                  </p>
                  <p className="text-sm text-slate-400">
                    When enabled, the bot will ask for confirmation before executing blockchain
                    transactions (transfers, swaps, contract calls). You will need to reply{' '}
                    <code className="text-stark-400">/confirm</code> to proceed or{' '}
                    <code className="text-stark-400">/cancel</code> to abort.
                  </p>
                </div>
              </div>
            </div>

            <div className="mt-4 flex items-center gap-2 text-sm">
              <span className={web3TxRequiresConfirmation ? 'text-green-400' : 'text-yellow-400'}>
                {web3TxRequiresConfirmation ? 'Confirmation required' : 'Auto-execute enabled'}
              </span>
              <span className="text-slate-500">
                {web3TxRequiresConfirmation
                  ? '- Transactions require manual approval'
                  : '- Transactions execute immediately (use with caution)'}
              </span>
            </div>
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
      </div>
    </div>
  );
}
