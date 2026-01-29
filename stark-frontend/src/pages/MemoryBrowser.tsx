import { useState, useEffect, useMemo } from 'react';
import {
  FileText,
  Trash2,
  Search,
  Filter,
  Download,
  Edit2,
  Check,
  X,
  ChevronDown,
  Merge,
  BarChart3,
  Clock,
  User,
  Tag,
  Star,
} from 'lucide-react';
import Card, { CardContent } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import {
  getMemoriesFiltered,
  deleteMemory,
  updateMemory,
  mergeMemories,
  getMemoryStats,
  exportMemories,
  searchMemories,
  MemoryInfo,
  MemoryStats,
  ListMemoriesParams,
} from '@/lib/api';

// Memory type labels and colors
const MEMORY_TYPES = {
  daily_log: { label: 'Daily Log', color: 'bg-blue-500/20 text-blue-400' },
  long_term: { label: 'Long Term', color: 'bg-purple-500/20 text-purple-400' },
  session_summary: { label: 'Session', color: 'bg-gray-500/20 text-gray-400' },
  compaction: { label: 'Compaction', color: 'bg-orange-500/20 text-orange-400' },
  preference: { label: 'Preference', color: 'bg-green-500/20 text-green-400' },
  fact: { label: 'Fact', color: 'bg-cyan-500/20 text-cyan-400' },
  entity: { label: 'Entity', color: 'bg-yellow-500/20 text-yellow-400' },
  task: { label: 'Task', color: 'bg-red-500/20 text-red-400' },
};

interface MemoryCardProps {
  memory: MemoryInfo;
  isSelected: boolean;
  isEditing: boolean;
  onSelect: () => void;
  onEdit: () => void;
  onSave: (updates: Partial<MemoryInfo>) => void;
  onCancel: () => void;
  onDelete: () => void;
}

function MemoryCard({
  memory,
  isSelected,
  isEditing,
  onSelect,
  onEdit,
  onSave,
  onCancel,
  onDelete,
}: MemoryCardProps) {
  const [editContent, setEditContent] = useState(memory.content);
  const [editImportance, setEditImportance] = useState(memory.importance);

  useEffect(() => {
    setEditContent(memory.content);
    setEditImportance(memory.importance);
  }, [memory, isEditing]);

  const typeConfig = MEMORY_TYPES[memory.memory_type as keyof typeof MEMORY_TYPES] || {
    label: memory.memory_type,
    color: 'bg-slate-500/20 text-slate-400',
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleString();
  };

  const handleSave = () => {
    onSave({
      content: editContent !== memory.content ? editContent : undefined,
      importance: editImportance !== memory.importance ? editImportance : undefined,
    });
  };

  return (
    <Card className={`transition-all ${isSelected ? 'ring-2 ring-stark-500' : ''}`}>
      <CardContent>
        <div className="flex items-start gap-4">
          {/* Selection checkbox */}
          <div className="pt-1">
            <input
              type="checkbox"
              checked={isSelected}
              onChange={onSelect}
              className="w-4 h-4 rounded border-slate-600 bg-slate-800 text-stark-500 focus:ring-stark-500"
            />
          </div>

          {/* Memory content */}
          <div className="flex-1 min-w-0">
            {/* Type badge and metadata */}
            <div className="flex items-center gap-2 mb-2 flex-wrap">
              <span className={`px-2 py-0.5 rounded text-xs font-medium ${typeConfig.color}`}>
                {typeConfig.label}
              </span>
              {memory.entity_type && (
                <span className="px-2 py-0.5 bg-slate-700 rounded text-xs text-slate-400">
                  <Tag className="w-3 h-3 inline mr-1" />
                  {memory.entity_type}
                  {memory.entity_name && `: ${memory.entity_name}`}
                </span>
              )}
              {memory.source_type === 'explicit' && (
                <span className="px-2 py-0.5 bg-green-500/10 rounded text-xs text-green-400">
                  Explicit
                </span>
              )}
              {memory.superseded_by && (
                <span className="px-2 py-0.5 bg-red-500/10 rounded text-xs text-red-400">
                  Superseded
                </span>
              )}
            </div>

            {/* Content */}
            {isEditing ? (
              <textarea
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-white text-sm resize-none"
                rows={4}
              />
            ) : (
              <p className="text-white whitespace-pre-wrap break-words text-sm">
                {memory.content}
              </p>
            )}

            {/* Footer metadata */}
            <div className="flex items-center gap-4 mt-3 text-xs text-slate-500 flex-wrap">
              <span className="flex items-center gap-1">
                <Clock className="w-3 h-3" />
                {formatDate(memory.created_at)}
              </span>
              {memory.identity_id && (
                <span className="flex items-center gap-1">
                  <User className="w-3 h-3" />
                  {memory.identity_id.slice(0, 8)}...
                </span>
              )}
              {memory.source_channel_type && (
                <span className="px-1.5 py-0.5 bg-slate-700 rounded">
                  {memory.source_channel_type}
                </span>
              )}
              {memory.valid_until && (
                <span className="flex items-center gap-1 text-yellow-500">
                  Expires: {new Date(memory.valid_until).toLocaleDateString()}
                </span>
              )}
              {/* Importance */}
              {isEditing ? (
                <div className="flex items-center gap-2">
                  <Star className="w-3 h-3" />
                  <input
                    type="range"
                    min="1"
                    max="10"
                    value={editImportance}
                    onChange={(e) => setEditImportance(parseInt(e.target.value))}
                    className="w-20 h-1"
                  />
                  <span>{editImportance}</span>
                </div>
              ) : (
                <span className="flex items-center gap-1">
                  <Star className="w-3 h-3" />
                  {memory.importance}
                </span>
              )}
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-1 shrink-0">
            {isEditing ? (
              <>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleSave}
                  className="text-green-400 hover:bg-green-500/20"
                >
                  <Check className="w-4 h-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onCancel}
                  className="text-slate-400 hover:bg-slate-700"
                >
                  <X className="w-4 h-4" />
                </Button>
              </>
            ) : (
              <>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onEdit}
                  className="text-slate-400 hover:text-white hover:bg-slate-700"
                >
                  <Edit2 className="w-4 h-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onDelete}
                  className="text-red-400 hover:text-red-300 hover:bg-red-500/20"
                >
                  <Trash2 className="w-4 h-4" />
                </Button>
              </>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default function MemoryBrowser() {
  const [memories, setMemories] = useState<MemoryInfo[]>([]);
  const [stats, setStats] = useState<MemoryStats | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedType, setSelectedType] = useState<string>('');
  const [minImportance, setMinImportance] = useState<number>(0);
  const [showSuperseded, setShowSuperseded] = useState(false);
  const [showFilters, setShowFilters] = useState(false);

  // Selection and editing
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [editingId, setEditingId] = useState<number | null>(null);

  // Merge modal
  const [showMergeModal, setShowMergeModal] = useState(false);
  const [mergeContent, setMergeContent] = useState('');

  // Stats modal
  const [showStats, setShowStats] = useState(false);

  // Pagination
  const [offset, setOffset] = useState(0);
  const limit = 50;

  // Load memories
  const loadMemories = async () => {
    try {
      setIsLoading(true);
      setError(null);

      let data: MemoryInfo[];

      if (searchQuery.trim()) {
        // Use search API
        const results = await searchMemories(searchQuery, {
          memory_type: selectedType || undefined,
          min_importance: minImportance || undefined,
          limit,
        });
        data = results.map((r) => r.memory);
      } else {
        // Use filtered list API
        const params: ListMemoriesParams = {
          limit,
          offset,
          include_superseded: showSuperseded,
        };
        if (selectedType) params.memory_type = selectedType;
        if (minImportance > 0) params.min_importance = minImportance;

        data = await getMemoriesFiltered(params);
      }

      setMemories(data);
    } catch (err) {
      setError('Failed to load memories');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  // Load stats
  const loadStats = async () => {
    try {
      const statsData = await getMemoryStats();
      setStats(statsData);
    } catch (err) {
      console.error('Failed to load stats:', err);
    }
  };

  useEffect(() => {
    loadMemories();
    loadStats();
  }, [selectedType, minImportance, showSuperseded, offset]);

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(() => {
      if (searchQuery !== '') {
        loadMemories();
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  const handleDelete = async (id: number) => {
    if (!confirm('Are you sure you want to delete this memory?')) return;
    try {
      await deleteMemory(String(id));
      setMemories((prev) => prev.filter((m) => m.id !== id));
      setSelectedIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    } catch (err) {
      setError('Failed to delete memory');
    }
  };

  const handleUpdate = async (id: number, updates: Partial<MemoryInfo>) => {
    try {
      const cleaned: Record<string, unknown> = {};
      if (updates.content !== undefined) cleaned.content = updates.content;
      if (updates.importance !== undefined) cleaned.importance = updates.importance;

      if (Object.keys(cleaned).length === 0) {
        setEditingId(null);
        return;
      }

      const updated = await updateMemory(id, cleaned);
      setMemories((prev) => prev.map((m) => (m.id === id ? updated : m)));
      setEditingId(null);
    } catch (err) {
      setError('Failed to update memory');
    }
  };

  const handleMerge = async () => {
    if (selectedIds.size < 2) {
      setError('Select at least 2 memories to merge');
      return;
    }
    if (!mergeContent.trim()) {
      setError('Please provide merged content');
      return;
    }

    try {
      const result = await mergeMemories(Array.from(selectedIds), mergeContent);
      // Remove superseded memories, add new one
      setMemories((prev) => [
        result.merged_memory,
        ...prev.filter((m) => !selectedIds.has(m.id)),
      ]);
      setSelectedIds(new Set());
      setShowMergeModal(false);
      setMergeContent('');
    } catch (err) {
      setError('Failed to merge memories');
    }
  };

  const handleExport = async () => {
    try {
      const markdown = await exportMemories();
      const blob = new Blob([markdown], { type: 'text/markdown' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'memories.md';
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setError('Failed to export memories');
    }
  };

  const toggleSelect = (id: number) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const selectAll = () => {
    if (selectedIds.size === memories.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(memories.map((m) => m.id)));
    }
  };

  // Calculate selected memories content for merge preview
  const selectedMemories = useMemo(() => {
    return memories.filter((m) => selectedIds.has(m.id));
  }, [memories, selectedIds]);

  if (isLoading && memories.length === 0) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading memories...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      {/* Header */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-white mb-1">Memory Browser</h1>
            <p className="text-slate-400 text-sm">
              {stats
                ? `${stats.total_count} total memories | ${stats.temporal_active_count} active`
                : 'Loading stats...'}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setShowStats(true)}
              className="flex items-center gap-2"
            >
              <BarChart3 className="w-4 h-4" />
              Stats
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={handleExport}
              className="flex items-center gap-2"
            >
              <Download className="w-4 h-4" />
              Export
            </Button>
          </div>
        </div>

        {/* Search and filters */}
        <div className="flex flex-col gap-3">
          <div className="flex gap-3">
            <div className="flex-1 relative">
              <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-500" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search memories..."
                className="w-full pl-10 pr-4 py-2 bg-slate-800 border border-slate-700 rounded-lg text-white text-sm focus:outline-none focus:border-stark-500"
              />
            </div>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setShowFilters(!showFilters)}
              className="flex items-center gap-2"
            >
              <Filter className="w-4 h-4" />
              Filters
              <ChevronDown
                className={`w-4 h-4 transition-transform ${showFilters ? 'rotate-180' : ''}`}
              />
            </Button>
          </div>

          {/* Expanded filters */}
          {showFilters && (
            <div className="flex flex-wrap gap-3 p-3 bg-slate-800/50 rounded-lg">
              <div className="flex items-center gap-2">
                <label className="text-sm text-slate-400">Type:</label>
                <select
                  value={selectedType}
                  onChange={(e) => setSelectedType(e.target.value)}
                  className="bg-slate-800 border border-slate-600 rounded px-2 py-1 text-sm text-white"
                >
                  <option value="">All Types</option>
                  {Object.entries(MEMORY_TYPES).map(([key, { label }]) => (
                    <option key={key} value={key}>
                      {label}
                    </option>
                  ))}
                </select>
              </div>
              <div className="flex items-center gap-2">
                <label className="text-sm text-slate-400">Min Importance:</label>
                <input
                  type="range"
                  min="0"
                  max="10"
                  value={minImportance}
                  onChange={(e) => setMinImportance(parseInt(e.target.value))}
                  className="w-24"
                />
                <span className="text-sm text-white w-4">{minImportance}</span>
              </div>
              <label className="flex items-center gap-2 text-sm text-slate-400 cursor-pointer">
                <input
                  type="checkbox"
                  checked={showSuperseded}
                  onChange={(e) => setShowSuperseded(e.target.checked)}
                  className="rounded border-slate-600 bg-slate-800 text-stark-500"
                />
                Show superseded
              </label>
            </div>
          )}
        </div>
      </div>

      {error && (
        <div className="mb-6 bg-red-500/20 border border-red-500/50 text-red-400 px-4 py-3 rounded-lg">
          {error}
          <button onClick={() => setError(null)} className="ml-2 underline">
            Dismiss
          </button>
        </div>
      )}

      {/* Selection actions */}
      {selectedIds.size > 0 && (
        <div className="mb-4 flex items-center gap-3 p-3 bg-stark-500/10 border border-stark-500/30 rounded-lg">
          <span className="text-sm text-stark-400">{selectedIds.size} selected</span>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setShowMergeModal(true)}
            disabled={selectedIds.size < 2}
            className="flex items-center gap-2"
          >
            <Merge className="w-4 h-4" />
            Merge
          </Button>
          <Button variant="secondary" size="sm" onClick={() => setSelectedIds(new Set())}>
            Clear
          </Button>
        </div>
      )}

      {/* Memory list */}
      {memories.length > 0 ? (
        <div className="space-y-3">
          {/* Select all */}
          <div className="flex items-center gap-2 px-2">
            <input
              type="checkbox"
              checked={selectedIds.size === memories.length && memories.length > 0}
              onChange={selectAll}
              className="w-4 h-4 rounded border-slate-600 bg-slate-800 text-stark-500"
            />
            <span className="text-sm text-slate-400">Select all</span>
          </div>

          {memories.map((memory) => (
            <MemoryCard
              key={memory.id}
              memory={memory}
              isSelected={selectedIds.has(memory.id)}
              isEditing={editingId === memory.id}
              onSelect={() => toggleSelect(memory.id)}
              onEdit={() => setEditingId(memory.id)}
              onSave={(updates) => handleUpdate(memory.id, updates)}
              onCancel={() => setEditingId(null)}
              onDelete={() => handleDelete(memory.id)}
            />
          ))}

          {/* Pagination */}
          <div className="flex justify-center gap-2 mt-6">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setOffset(Math.max(0, offset - limit))}
              disabled={offset === 0}
            >
              Previous
            </Button>
            <span className="px-4 py-2 text-sm text-slate-400">
              Showing {offset + 1} - {offset + memories.length}
            </span>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setOffset(offset + limit)}
              disabled={memories.length < limit}
            >
              Next
            </Button>
          </div>
        </div>
      ) : (
        <Card>
          <CardContent className="text-center py-12">
            <FileText className="w-12 h-12 text-slate-600 mx-auto mb-4" />
            <p className="text-slate-400">No memories found</p>
          </CardContent>
        </Card>
      )}

      {/* Merge Modal */}
      {showMergeModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-slate-900 rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto">
            <h2 className="text-xl font-bold text-white mb-4">Merge Memories</h2>

            <div className="mb-4">
              <h3 className="text-sm font-medium text-slate-400 mb-2">
                Selected memories ({selectedMemories.length}):
              </h3>
              <div className="space-y-2 max-h-40 overflow-y-auto">
                {selectedMemories.map((m) => (
                  <div key={m.id} className="p-2 bg-slate-800 rounded text-sm text-slate-300">
                    {m.content.slice(0, 100)}...
                  </div>
                ))}
              </div>
            </div>

            <div className="mb-4">
              <label className="text-sm font-medium text-slate-400 mb-2 block">
                Merged content:
              </label>
              <textarea
                value={mergeContent}
                onChange={(e) => setMergeContent(e.target.value)}
                placeholder="Enter the consolidated content for the merged memory..."
                className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-white text-sm resize-none"
                rows={6}
              />
            </div>

            <div className="flex justify-end gap-2">
              <Button
                variant="secondary"
                onClick={() => {
                  setShowMergeModal(false);
                  setMergeContent('');
                }}
              >
                Cancel
              </Button>
              <Button onClick={handleMerge}>Merge Memories</Button>
            </div>
          </div>
        </div>
      )}

      {/* Stats Modal */}
      {showStats && stats && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-slate-900 rounded-lg p-6 max-w-lg w-full mx-4">
            <h2 className="text-xl font-bold text-white mb-4">Memory Statistics</h2>

            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="p-3 bg-slate-800 rounded">
                  <div className="text-2xl font-bold text-stark-400">{stats.total_count}</div>
                  <div className="text-sm text-slate-400">Total Memories</div>
                </div>
                <div className="p-3 bg-slate-800 rounded">
                  <div className="text-2xl font-bold text-green-400">
                    {stats.temporal_active_count}
                  </div>
                  <div className="text-sm text-slate-400">Active</div>
                </div>
                <div className="p-3 bg-slate-800 rounded">
                  <div className="text-2xl font-bold text-orange-400">
                    {stats.superseded_count}
                  </div>
                  <div className="text-sm text-slate-400">Superseded</div>
                </div>
                <div className="p-3 bg-slate-800 rounded">
                  <div className="text-2xl font-bold text-purple-400">
                    {stats.avg_importance.toFixed(1)}
                  </div>
                  <div className="text-sm text-slate-400">Avg Importance</div>
                </div>
              </div>

              <div>
                <h3 className="text-sm font-medium text-slate-400 mb-2">By Type:</h3>
                <div className="grid grid-cols-2 gap-2">
                  {Object.entries(stats.by_type).map(([type, count]) => {
                    const config = MEMORY_TYPES[type as keyof typeof MEMORY_TYPES];
                    return (
                      <div key={type} className="flex justify-between text-sm">
                        <span className={config?.color || 'text-slate-400'}>
                          {config?.label || type}
                        </span>
                        <span className="text-white">{count}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>

            <div className="mt-6 flex justify-end">
              <Button variant="secondary" onClick={() => setShowStats(false)}>
                Close
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
