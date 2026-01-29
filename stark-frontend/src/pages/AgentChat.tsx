import { useState, useEffect, useRef, useCallback, KeyboardEvent } from 'react';
import { Send, RotateCcw, Copy, Check, Wallet, Bug } from 'lucide-react';
import Button from '@/components/ui/Button';
import ChatMessage from '@/components/chat/ChatMessage';
import TypingIndicator from '@/components/chat/TypingIndicator';
import ExecutionProgress from '@/components/chat/ExecutionProgress';
import DebugPanel from '@/components/chat/DebugPanel';
import CommandAutocomplete from '@/components/chat/CommandAutocomplete';
import TransactionTracker from '@/components/chat/TransactionTracker';
import { ConfirmationPrompt } from '@/components/chat/ConfirmationPrompt';
import { useGateway } from '@/hooks/useGateway';
import { useWallet } from '@/hooks/useWallet';
import { sendChatMessage, getAgentSettings, getSkills, getTools, confirmTransaction, cancelTransaction } from '@/lib/api';
import type { ChatMessage as ChatMessageType, MessageRole, SlashCommand, TrackedTransaction, TxPendingEvent, TxConfirmedEvent, PendingConfirmation, ConfirmationRequiredEvent } from '@/types';

interface ConversationMessage {
  role: string;
  content: string;
}

export default function AgentChat() {
  const [messages, setMessages] = useState<ChatMessageType[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [showAutocomplete, setShowAutocomplete] = useState(false);
  const [selectedCommandIndex, setSelectedCommandIndex] = useState(0);
  const [debugMode, setDebugMode] = useState(false);
  const [sessionStartTime] = useState(new Date());
  const [copied, setCopied] = useState(false);
  const [trackedTxs, setTrackedTxs] = useState<TrackedTransaction[]>([]);
  const [pendingConfirmation, setPendingConfirmation] = useState<PendingConfirmation | null>(null);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const { connected, on, off } = useGateway();
  const { address, usdcBalance, isConnected: walletConnected, connect: connectWallet, isCorrectNetwork } = useWallet();

  // Helper to truncate address
  const truncateAddress = (addr: string) => `${addr.slice(0, 6)}...${addr.slice(-4)}`;

  // Copy address to clipboard
  const copyAddress = useCallback(() => {
    if (address) {
      navigator.clipboard.writeText(address);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }, [address]);

  // Format USDC balance
  const formatBalance = (balance: string | null) => {
    if (!balance) return '0.00';
    const num = parseFloat(balance);
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(2)}K`;
    return num.toFixed(2);
  };

  // Conversation history for API
  const conversationHistory = useRef<ConversationMessage[]>([]);

  // Define slash commands
  const slashCommands: SlashCommand[] = [
    {
      name: 'help',
      description: 'List all available commands',
      handler: () => {
        const helpText = slashCommands
          .map((cmd) => `• **/${cmd.name}** - ${cmd.description}`)
          .join('\n');
        addMessage('system', `**Available Commands:**\n\n${helpText}`);
      },
    },
    {
      name: 'status',
      description: 'Show session statistics',
      handler: async () => {
        const settings = await getAgentSettings();
        const duration = Math.floor((Date.now() - sessionStartTime.getTime()) / 1000);
        const mins = Math.floor(duration / 60);
        const secs = duration % 60;
        const messageCount = conversationHistory.current.length;
        const tokenEstimate = conversationHistory.current
          .reduce((acc, m) => acc + Math.ceil(m.content.length / 4), 0);

        addMessage('system', `**Session Status:**\n\n• Messages: ${messageCount}\n• Duration: ${mins}m ${secs}s\n• Provider: ${(settings as Record<string, unknown>).provider || 'anthropic'}\n• Est. tokens: ~${tokenEstimate}`);
      },
    },
    {
      name: 'new',
      description: 'Start a new conversation',
      handler: () => {
        setMessages([]);
        conversationHistory.current = [];
        addMessage('system', 'Conversation cleared. Starting fresh.');
      },
    },
    {
      name: 'reset',
      description: 'Reset conversation history',
      handler: () => {
        setMessages([]);
        conversationHistory.current = [];
        addMessage('system', 'Conversation reset.');
      },
    },
    {
      name: 'clear',
      description: 'Clear the chat display',
      handler: () => {
        setMessages([]);
        conversationHistory.current = [];
      },
    },
    {
      name: 'skills',
      description: 'List available skills',
      handler: async () => {
        try {
          const skills = await getSkills();
          if (skills.length === 0) {
            addMessage('system', 'No skills installed.');
            return;
          }
          const skillList = skills
            .map((s) => `• **${s.name}** - ${s.description || 'No description'}`)
            .join('\n');
          addMessage('system', `**Available Skills:**\n\n${skillList}`);
        } catch {
          addMessage('error', 'Failed to load skills');
        }
      },
    },
    {
      name: 'tools',
      description: 'List available tools',
      handler: async () => {
        try {
          const tools = await getTools();
          if (tools.length === 0) {
            addMessage('system', 'No tools available.');
            return;
          }
          const toolList = tools
            .map((t) => `• **${t.name}** ${t.enabled ? '✓' : '✗'} - ${t.description || 'No description'}`)
            .join('\n');
          addMessage('system', `**Available Tools:**\n\n${toolList}`);
        } catch {
          addMessage('error', 'Failed to load tools');
        }
      },
    },
    {
      name: 'model',
      description: 'Show model configuration',
      handler: async () => {
        try {
          const settings = await getAgentSettings() as Record<string, unknown>;
          addMessage('system', `**Model Configuration:**\n\n• Provider: ${settings.provider || 'anthropic'}\n• Model: ${settings.model || 'claude-3-opus'}\n• Temperature: ${settings.temperature ?? 0.7}`);
        } catch {
          addMessage('error', 'Failed to load model configuration');
        }
      },
    },
    {
      name: 'export',
      description: 'Download conversation as JSON',
      handler: () => {
        const data = {
          messages: conversationHistory.current,
          exportedAt: new Date().toISOString(),
          sessionStart: sessionStartTime.toISOString(),
        };
        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `chat-export-${Date.now()}.json`;
        a.click();
        URL.revokeObjectURL(url);
        addMessage('system', 'Conversation exported.');
      },
    },
    {
      name: 'debug',
      description: 'Toggle debug mode',
      handler: () => {
        setDebugMode((prev) => !prev);
        addMessage('system', `Debug mode ${!debugMode ? 'enabled' : 'disabled'}.`);
      },
    },
    {
      name: 'confirm',
      description: 'Confirm pending transaction',
      handler: async () => {
        if (!pendingConfirmation) {
          addMessage('system', 'No pending transaction to confirm.');
          return;
        }
        try {
          addMessage('system', 'Confirming transaction...');
          const result = await confirmTransaction(pendingConfirmation.channel_id);
          if (result.success) {
            addMessage('system', result.message || 'Transaction confirmed and executing.');
            setPendingConfirmation(null);
          } else {
            addMessage('error', result.error || 'Failed to confirm transaction.');
          }
        } catch (error) {
          addMessage('error', error instanceof Error ? error.message : 'Failed to confirm transaction');
        }
      },
    },
    {
      name: 'cancel',
      description: 'Cancel pending transaction',
      handler: async () => {
        if (!pendingConfirmation) {
          addMessage('system', 'No pending transaction to cancel.');
          return;
        }
        try {
          const result = await cancelTransaction(pendingConfirmation.channel_id);
          if (result.success) {
            addMessage('system', result.message || 'Transaction cancelled.');
            setPendingConfirmation(null);
          } else {
            addMessage('error', result.error || 'Failed to cancel transaction.');
          }
        } catch (error) {
          addMessage('error', error instanceof Error ? error.message : 'Failed to cancel transaction');
        }
      },
    },
  ];

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, scrollToBottom]);

  // Listen for real-time tool call events from the agent
  useEffect(() => {
    const handleToolCall = (data: unknown) => {
      const event = data as { tool_name: string; parameters: Record<string, unknown> };
      const paramsPretty = JSON.stringify(event.parameters, null, 2);
      const content = `🔧 **Tool Call:** \`${event.tool_name}\`\n\`\`\`json\n${paramsPretty}\n\`\`\``;

      const message: ChatMessageType = {
        id: crypto.randomUUID(),
        role: 'tool' as MessageRole,
        content,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, message]);
    };

    on('agent.tool_call', handleToolCall);
    return () => {
      off('agent.tool_call', handleToolCall);
    };
  }, [on, off]);

  // Listen for tool result events to show success/failure in chat
  useEffect(() => {
    const handleToolResult = (data: unknown) => {
      const event = data as { tool_name: string; success: boolean; duration_ms: number; content: string };
      const statusEmoji = event.success ? '✅' : '❌';
      const statusText = event.success ? 'Success' : 'Failed';

      // Truncate content if too long for chat display
      let displayContent = event.content;
      if (displayContent.length > 2000) {
        displayContent = displayContent.substring(0, 2000) + '\n... (truncated)';
      }

      const content = `${statusEmoji} **Tool Result:** \`${event.tool_name}\` - ${statusText} (${event.duration_ms}ms)\n\`\`\`\n${displayContent}\n\`\`\``;

      const message: ChatMessageType = {
        id: crypto.randomUUID(),
        role: event.success ? 'tool' as MessageRole : 'error' as MessageRole,
        content,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, message]);
    };

    on('tool.result', handleToolResult);
    return () => {
      off('tool.result', handleToolResult);
    };
  }, [on, off]);

  // Listen for transaction events
  useEffect(() => {
    const handleTxPending = (data: unknown) => {
      const event = data as TxPendingEvent;
      console.log('[TX] Pending transaction:', event.tx_hash);

      setTrackedTxs((prev) => {
        // Avoid duplicates
        if (prev.some((tx) => tx.tx_hash === event.tx_hash)) {
          return prev;
        }
        return [
          ...prev,
          {
            tx_hash: event.tx_hash,
            network: event.network,
            explorer_url: event.explorer_url,
            status: 'pending',
            timestamp: new Date(),
          },
        ];
      });
    };

    const handleTxConfirmed = (data: unknown) => {
      const event = data as TxConfirmedEvent;
      console.log('[TX] Transaction confirmed:', event.tx_hash, event.status);

      setTrackedTxs((prev) =>
        prev.map((tx) =>
          tx.tx_hash === event.tx_hash
            ? { ...tx, status: event.status as 'confirmed' | 'reverted' | 'pending' }
            : tx
        )
      );

      // Auto-remove confirmed transactions after 30 seconds
      if (event.status === 'confirmed') {
        setTimeout(() => {
          setTrackedTxs((prev) => prev.filter((tx) => tx.tx_hash !== event.tx_hash));
        }, 30000);
      }
    };

    on('tx.pending', handleTxPending);
    on('tx.confirmed', handleTxConfirmed);

    return () => {
      off('tx.pending', handleTxPending);
      off('tx.confirmed', handleTxConfirmed);
    };
  }, [on, off]);

  // Listen for confirmation events
  useEffect(() => {
    const handleConfirmationRequired = (data: unknown) => {
      const event = data as ConfirmationRequiredEvent;
      console.log('[Confirmation] Required:', event.tool_name, event.description);

      setPendingConfirmation({
        confirmation_id: event.confirmation_id,
        channel_id: event.channel_id,
        tool_name: event.tool_name,
        description: event.description,
        parameters: event.parameters,
        timestamp: event.timestamp,
      });
    };

    const handleConfirmationApproved = () => {
      console.log('[Confirmation] Approved');
      setPendingConfirmation(null);
    };

    const handleConfirmationRejected = () => {
      console.log('[Confirmation] Rejected');
      setPendingConfirmation(null);
    };

    on('confirmation.required', handleConfirmationRequired);
    on('confirmation.approved', handleConfirmationApproved);
    on('confirmation.rejected', handleConfirmationRejected);

    return () => {
      off('confirmation.required', handleConfirmationRequired);
      off('confirmation.approved', handleConfirmationApproved);
      off('confirmation.rejected', handleConfirmationRejected);
    };
  }, [on, off]);

  const addMessage = useCallback((role: MessageRole, content: string) => {
    const message: ChatMessageType = {
      id: crypto.randomUUID(),
      role,
      content,
      timestamp: new Date(),
    };
    setMessages((prev) => [...prev, message]);

    // Add to conversation history if user or assistant
    if (role === 'user' || role === 'assistant') {
      conversationHistory.current.push({ role, content });
    }
  }, []);

  const handleCommand = useCallback(async (commandName: string) => {
    const command = slashCommands.find((c) => c.name === commandName);
    if (command) {
      addMessage('command', `/${commandName}`);
      await command.handler();
    } else {
      addMessage('error', `Unknown command: /${commandName}`);
    }
  }, [addMessage, slashCommands]);

  const handleSend = useCallback(async () => {
    const trimmedInput = input.trim();
    if (!trimmedInput || isLoading) return;

    setInput('');
    setShowAutocomplete(false);

    // Handle slash commands
    if (trimmedInput.startsWith('/')) {
      const commandName = trimmedInput.slice(1).split(' ')[0];
      await handleCommand(commandName);
      return;
    }

    // Regular message
    addMessage('user', trimmedInput);
    setIsLoading(true);

    try {
      const response = await sendChatMessage(trimmedInput, conversationHistory.current);
      addMessage('assistant', response.response);
    } catch (error) {
      addMessage('error', error instanceof Error ? error.message : 'Failed to send message');
    } finally {
      setIsLoading(false);
    }
  }, [input, isLoading, addMessage, handleCommand]);

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLTextAreaElement>) => {
    // Handle autocomplete navigation
    if (showAutocomplete) {
      const filteredCommands = slashCommands.filter((cmd) =>
        cmd.name.toLowerCase().startsWith(input.slice(1).toLowerCase())
      );

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedCommandIndex((prev) =>
          prev < filteredCommands.length - 1 ? prev + 1 : prev
        );
        return;
      }

      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedCommandIndex((prev) => (prev > 0 ? prev - 1 : 0));
        return;
      }

      if (e.key === 'Tab' || e.key === 'Enter') {
        if (filteredCommands.length > 0) {
          e.preventDefault();
          const selectedCommand = filteredCommands[selectedCommandIndex];
          setInput(`/${selectedCommand.name}`);
          setShowAutocomplete(false);
          if (e.key === 'Enter') {
            handleCommand(selectedCommand.name);
            setInput('');
          }
          return;
        }
      }

      if (e.key === 'Escape') {
        setShowAutocomplete(false);
        return;
      }
    }

    // Send on Enter (without shift)
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }, [showAutocomplete, input, selectedCommandIndex, slashCommands, handleSend, handleCommand]);

  const handleInputChange = useCallback((value: string) => {
    setInput(value);

    // Show autocomplete for slash commands
    if (value.startsWith('/') && !value.includes(' ')) {
      setShowAutocomplete(true);
      setSelectedCommandIndex(0);
    } else {
      setShowAutocomplete(false);
    }
  }, []);

  const handleCommandSelect = useCallback((command: SlashCommand) => {
    setInput('');
    setShowAutocomplete(false);
    handleCommand(command.name);
  }, [handleCommand]);

  return (
    <div className="flex flex-col h-screen">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-slate-700 bg-slate-800/50">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-bold text-white">Agent Chat</h1>
          <div className="flex items-center gap-2">
            <span
              className={`w-2 h-2 rounded-full ${
                connected ? 'bg-green-400' : 'bg-red-400'
              }`}
            />
            <span className="text-sm text-slate-400">
              {connected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
        </div>

        {/* Debug Toggle + Wallet Info */}
        <div className="flex items-center gap-4">
          {/* Debug Toggle - to the left of wallet */}
          <button
            onClick={() => setDebugMode(!debugMode)}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
              debugMode
                ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/50'
                : 'bg-slate-700/50 text-slate-400 hover:text-slate-200 hover:bg-slate-700'
            }`}
            title="Toggle debug mode"
          >
            <Bug className="w-4 h-4" />
            <span className="hidden sm:inline">Debug</span>
            {/* Toggle switch */}
            <div className={`w-8 h-4 rounded-full transition-colors ${debugMode ? 'bg-cyan-500' : 'bg-slate-600'}`}>
              <div
                className={`w-3 h-3 rounded-full bg-white transition-transform transform mt-0.5 ${
                  debugMode ? 'translate-x-4 ml-0.5' : 'translate-x-0.5'
                }`}
              />
            </div>
          </button>

          {walletConnected && address ? (
            <div className="flex items-center gap-3">
              {/* Wallet Address */}
              <div className="flex items-center gap-2 bg-slate-700/50 px-3 py-1.5 rounded-lg">
                <span className="text-sm font-mono text-slate-300">
                  {truncateAddress(address)}
                </span>
                <button
                  onClick={copyAddress}
                  className="text-slate-400 hover:text-slate-200 transition-colors"
                  title="Copy address"
                >
                  {copied ? (
                    <Check className="w-4 h-4 text-green-400" />
                  ) : (
                    <Copy className="w-4 h-4" />
                  )}
                </button>
              </div>

              {/* USDC Balance */}
              <div className="flex items-center gap-2 bg-slate-700/50 px-3 py-1.5 rounded-lg">
                <span className="text-sm font-semibold text-white">
                  {isCorrectNetwork ? formatBalance(usdcBalance) : '--'}
                </span>
                <span className="text-xs px-2 py-0.5 bg-blue-500/20 text-blue-400 rounded-full font-medium">
                  USDC · Base
                </span>
              </div>
            </div>
          ) : (
            <button
              onClick={connectWallet}
              className="flex items-center gap-2 bg-stark-500/20 hover:bg-stark-500/30 text-stark-400 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors"
            >
              <Wallet className="w-4 h-4" />
              Connect Wallet
            </button>
          )}

          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              setMessages([]);
              conversationHistory.current = [];
            }}
          >
            <RotateCcw className="w-4 h-4 mr-2" />
            Clear
          </Button>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-6">
        {messages.length === 0 ? (
          <div className="h-full flex items-center justify-center">
            <div className="text-center">
              <h2 className="text-xl font-semibold text-white mb-2">
                Welcome to Agent Chat
              </h2>
              <p className="text-slate-400 mb-4">
                Start a conversation or type <code className="bg-slate-700 px-1 rounded">/help</code> for commands
              </p>
            </div>
          </div>
        ) : (
          <>
            {messages.map((message) => (
              <ChatMessage
                key={message.id}
                role={message.role}
                content={message.content}
                timestamp={message.timestamp}
              />
            ))}
            {isLoading && <TypingIndicator />}
          </>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Debug Panel - always mounted to capture events, hidden when not in debug mode */}
      <DebugPanel className={`mx-6 mb-4 ${debugMode ? '' : 'hidden'}`} />

      {/* Execution Progress */}
      <ExecutionProgress className="mx-6 mb-4" />

      {/* Transaction Tracker */}
      <TransactionTracker transactions={trackedTxs} className="mx-6 mb-4" />

      {/* Confirmation Prompt */}
      {pendingConfirmation && (
        <div className="mx-6 mb-4">
          <ConfirmationPrompt
            confirmation={pendingConfirmation}
            onConfirm={async (confirmationId) => {
              console.log('[Confirmation] Confirming:', confirmationId);
              const result = await confirmTransaction(pendingConfirmation.channel_id);
              if (result.success) {
                addMessage('system', result.message || 'Transaction confirmed and executing.');
                setPendingConfirmation(null);
              } else {
                throw new Error(result.error || 'Failed to confirm');
              }
            }}
            onCancel={async (confirmationId) => {
              console.log('[Confirmation] Cancelling:', confirmationId);
              const result = await cancelTransaction(pendingConfirmation.channel_id);
              if (result.success) {
                addMessage('system', result.message || 'Transaction cancelled.');
                setPendingConfirmation(null);
              } else {
                throw new Error(result.error || 'Failed to cancel');
              }
            }}
          />
        </div>
      )}

      {/* Input */}
      <div className="px-6 pb-6">
        <div className="relative">
          {showAutocomplete && (
            <CommandAutocomplete
              commands={slashCommands}
              filter={input}
              selectedIndex={selectedCommandIndex}
              onSelect={handleCommandSelect}
              onClose={() => setShowAutocomplete(false)}
            />
          )}
          <div className="flex gap-3">
            <div className="flex-1 relative">
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => handleInputChange(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Type a message or /command..."
                rows={1}
                className="w-full px-4 py-3 bg-slate-800 border border-slate-700 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-stark-500 focus:border-transparent resize-none"
                style={{ minHeight: '48px', maxHeight: '200px' }}
              />
            </div>
            <Button
              onClick={handleSend}
              disabled={!input.trim() || isLoading}
              className="shrink-0"
            >
              <Send className="w-5 h-5" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
