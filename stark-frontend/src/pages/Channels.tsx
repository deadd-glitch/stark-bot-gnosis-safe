import { useState } from 'react';
import { MessageSquare, Hash } from 'lucide-react';
import Card, { CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import Input from '@/components/ui/Input';

export default function Channels() {
  const [isLoading] = useState(false);
  const [error] = useState<string | null>(null);

  if (isLoading) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading channels...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-white mb-2">Channels</h1>
        <p className="text-slate-400">Configure messaging platform integrations</p>
      </div>

      {error && (
        <div className="mb-6 bg-red-500/20 border border-red-500/50 text-red-400 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      <div className="grid gap-6">
        {/* Telegram */}
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="p-2 bg-blue-500/20 rounded-lg">
                <MessageSquare className="w-5 h-5 text-blue-400" />
              </div>
              <CardTitle>Telegram</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <Input
                label="Bot Token"
                type="password"
                placeholder="Enter your Telegram bot token"
              />
              <div className="flex items-center justify-between">
                <p className="text-sm text-slate-400">
                  Get a token from @BotFather on Telegram
                </p>
                <Button variant="secondary" size="sm">
                  Save
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Slack */}
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="p-2 bg-purple-500/20 rounded-lg">
                <Hash className="w-5 h-5 text-purple-400" />
              </div>
              <CardTitle>Slack</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <Input
                label="Bot Token"
                type="password"
                placeholder="xoxb-..."
              />
              <Input
                label="Signing Secret"
                type="password"
                placeholder="Enter signing secret"
              />
              <div className="flex items-center justify-between">
                <p className="text-sm text-slate-400">
                  Configure in your Slack app settings
                </p>
                <Button variant="secondary" size="sm">
                  Save
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Discord */}
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="p-2 bg-indigo-500/20 rounded-lg">
                <Hash className="w-5 h-5 text-indigo-400" />
              </div>
              <CardTitle>Discord</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <Input
                label="Bot Token"
                type="password"
                placeholder="Enter your Discord bot token"
              />
              <div className="flex items-center justify-between">
                <p className="text-sm text-slate-400">
                  Get a token from the Discord Developer Portal
                </p>
                <Button variant="secondary" size="sm">
                  Save
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
