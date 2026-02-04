import { useEffect, useRef, useState, useCallback } from 'react';
import * as d3 from 'd3';
import { X, Save, Trash2 } from 'lucide-react';
import Button from '@/components/ui/Button';
import {
  getMindGraph,
  createMindNode,
  updateMindNode,
  deleteMindNode,
  MindNodeInfo,
  MindConnectionInfo,
} from '@/lib/api';

interface D3Node extends d3.SimulationNodeDatum {
  id: number;
  body: string;
  is_trunk: boolean;
  fx?: number | null;
  fy?: number | null;
}

interface D3Link extends d3.SimulationLinkDatum<D3Node> {
  source: D3Node | number;
  target: D3Node | number;
}

export default function MindMap() {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const simulationRef = useRef<d3.Simulation<D3Node, D3Link> | null>(null);

  const [nodes, setNodes] = useState<MindNodeInfo[]>([]);
  const [connections, setConnections] = useState<MindConnectionInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Modal state for editing node body
  const [editingNode, setEditingNode] = useState<MindNodeInfo | null>(null);
  const [editBody, setEditBody] = useState('');

  // Load graph data
  const loadGraph = useCallback(async () => {
    try {
      setLoading(true);
      const graph = await getMindGraph();
      setNodes(graph.nodes);
      setConnections(graph.connections);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load mind map');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadGraph();
  }, [loadGraph]);

  // Handle click on node to create child
  const handleNodeClick = useCallback(async (node: D3Node) => {
    try {
      await createMindNode({ parent_id: node.id });
      await loadGraph();
    } catch (e) {
      console.error('Failed to create node:', e);
    }
  }, [loadGraph]);

  // Handle right-click to edit node
  const handleNodeRightClick = useCallback((event: MouseEvent, node: D3Node) => {
    event.preventDefault();
    const nodeInfo = nodes.find(n => n.id === node.id);
    if (nodeInfo) {
      setEditingNode(nodeInfo);
      setEditBody(nodeInfo.body);
    }
  }, [nodes]);

  // Handle save edit
  const handleSaveEdit = async () => {
    if (!editingNode) return;
    try {
      await updateMindNode(editingNode.id, { body: editBody });
      setEditingNode(null);
      await loadGraph();
    } catch (e) {
      console.error('Failed to update node:', e);
    }
  };

  // Handle delete node
  const handleDeleteNode = async () => {
    if (!editingNode || editingNode.is_trunk) return;
    try {
      await deleteMindNode(editingNode.id);
      setEditingNode(null);
      await loadGraph();
    } catch (e) {
      console.error('Failed to delete node:', e);
    }
  };

  // Handle drag to update position
  const handleDragEnd = useCallback(async (node: D3Node) => {
    if (node.x !== undefined && node.y !== undefined) {
      try {
        await updateMindNode(node.id, {
          position_x: node.x,
          position_y: node.y,
        });
      } catch (e) {
        console.error('Failed to update position:', e);
      }
    }
  }, []);

  // D3 visualization
  useEffect(() => {
    if (loading || !svgRef.current || !containerRef.current || nodes.length === 0) return;

    const svg = d3.select(svgRef.current);
    const container = containerRef.current;
    const width = container.clientWidth;
    const height = container.clientHeight;

    // Clear previous content
    svg.selectAll('*').remove();

    // Create main group for zoom/pan
    const g = svg.append('g');

    // Setup zoom
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 4])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });

    svg.call(zoom);

    // Center the view initially
    svg.call(zoom.transform, d3.zoomIdentity.translate(width / 2, height / 2));

    // Prepare data for D3
    const d3Nodes: D3Node[] = nodes.map(n => ({
      id: n.id,
      body: n.body,
      is_trunk: n.is_trunk,
      x: n.position_x ?? undefined,
      y: n.position_y ?? undefined,
    }));

    const d3Links: D3Link[] = connections.map(c => ({
      source: c.parent_id,
      target: c.child_id,
    }));

    // Create simulation
    const simulation = d3.forceSimulation<D3Node, D3Link>(d3Nodes)
      .force('link', d3.forceLink<D3Node, D3Link>(d3Links)
        .id(d => d.id)
        .distance(100)
        .strength(0.5))
      .force('charge', d3.forceManyBody().strength(-300))
      .force('center', d3.forceCenter(0, 0))
      .force('collide', d3.forceCollide().radius(40));

    simulationRef.current = simulation;

    // Draw links
    const link = g.append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(d3Links)
      .join('line')
      .attr('stroke', '#444')
      .attr('stroke-width', 2)
      .attr('stroke-opacity', 0.6);

    // Draw nodes
    const node = g.append('g')
      .attr('class', 'nodes')
      .selectAll('g')
      .data(d3Nodes)
      .join('g')
      .attr('cursor', 'pointer');

    // Node circles
    node.append('circle')
      .attr('r', d => d.is_trunk ? 30 : 20)
      .attr('fill', d => d.is_trunk ? '#22c55e' : '#ffffff')
      .attr('stroke', d => d.is_trunk ? '#16a34a' : '#888')
      .attr('stroke-width', 2)
      .style('transition', 'r 0.2s ease, fill 0.2s ease');

    // Node labels (body text preview)
    node.append('text')
      .text(d => d.body.slice(0, 10) + (d.body.length > 10 ? '...' : ''))
      .attr('text-anchor', 'middle')
      .attr('dy', d => d.is_trunk ? 50 : 35)
      .attr('fill', '#888')
      .attr('font-size', '12px')
      .style('pointer-events', 'none');

    // Hover effects
    node.on('mouseenter', function(_event, d) {
      d3.select(this).select('circle')
        .transition()
        .duration(200)
        .attr('r', d.is_trunk ? 35 : 25)
        .attr('fill', d.is_trunk ? '#4ade80' : '#e0e0e0');
    })
    .on('mouseleave', function(_event, d) {
      d3.select(this).select('circle')
        .transition()
        .duration(200)
        .attr('r', d.is_trunk ? 30 : 20)
        .attr('fill', d.is_trunk ? '#22c55e' : '#ffffff');
    });

    // Click handler for creating children
    node.on('click', (event, d) => {
      event.stopPropagation();
      handleNodeClick(d);
    });

    // Right-click handler for editing
    node.on('contextmenu', (event: MouseEvent, d: D3Node) => {
      handleNodeRightClick(event, d);
    });

    // Drag behavior
    const drag = d3.drag<SVGGElement, D3Node>()
      .on('start', (event, d) => {
        if (!event.active) simulation.alphaTarget(0.3).restart();
        d.fx = d.x;
        d.fy = d.y;
      })
      .on('drag', (event, d) => {
        d.fx = event.x;
        d.fy = event.y;
      })
      .on('end', (event, d) => {
        if (!event.active) simulation.alphaTarget(0);
        // Keep position fixed after drag
        handleDragEnd(d);
      });

    (node as d3.Selection<SVGGElement, D3Node, SVGGElement, unknown>).call(drag);

    // Update positions on tick
    simulation.on('tick', () => {
      link
        .attr('x1', d => (d.source as D3Node).x ?? 0)
        .attr('y1', d => (d.source as D3Node).y ?? 0)
        .attr('x2', d => (d.target as D3Node).x ?? 0)
        .attr('y2', d => (d.target as D3Node).y ?? 0);

      node.attr('transform', d => `translate(${d.x ?? 0},${d.y ?? 0})`);
    });

    // Cleanup
    return () => {
      simulation.stop();
    };
  }, [loading, nodes, connections, handleNodeClick, handleNodeRightClick, handleDragEnd]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full bg-black">
        <div className="text-gray-400">Loading mind map...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full bg-black">
        <div className="text-red-400">{error}</div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-black">
      {/* Header */}
      <div className="p-4 border-b border-gray-800 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-white">Mind Map</h1>
          <p className="text-sm text-gray-400">
            Click a node to add child. Right-click to edit. Drag to reposition. Scroll to zoom.
          </p>
        </div>
        <div className="text-sm text-gray-500">
          {nodes.length} nodes, {connections.length} connections
        </div>
      </div>

      {/* Canvas */}
      <div ref={containerRef} className="flex-1 relative overflow-hidden">
        <svg
          ref={svgRef}
          className="w-full h-full"
          style={{ background: '#000' }}
        />
      </div>

      {/* Edit Modal */}
      {editingNode && (
        <div className="fixed inset-0 bg-black/80 flex items-center justify-center z-50">
          <div className="bg-gray-900 rounded-lg p-6 w-full max-w-md mx-4 border border-gray-700">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-white">
                {editingNode.is_trunk ? 'Edit Trunk Node' : 'Edit Node'}
              </h2>
              <button
                onClick={() => setEditingNode(null)}
                className="text-gray-400 hover:text-white"
              >
                <X size={20} />
              </button>
            </div>

            <textarea
              value={editBody}
              onChange={(e) => setEditBody(e.target.value)}
              className="w-full h-32 bg-gray-800 border border-gray-600 rounded-lg p-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-stark-500 resize-none"
              placeholder="Enter node content..."
              autoFocus
            />

            <div className="flex items-center justify-between mt-4">
              {!editingNode.is_trunk && (
                <Button
                  variant="ghost"
                  onClick={handleDeleteNode}
                  className="text-red-400 hover:text-red-300 hover:bg-red-500/10"
                >
                  <Trash2 size={16} className="mr-2" />
                  Delete
                </Button>
              )}
              <div className={`flex gap-2 ${editingNode.is_trunk ? 'ml-auto' : ''}`}>
                <Button variant="secondary" onClick={() => setEditingNode(null)}>
                  Cancel
                </Button>
                <Button variant="primary" onClick={handleSaveEdit}>
                  <Save size={16} className="mr-2" />
                  Save
                </Button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
