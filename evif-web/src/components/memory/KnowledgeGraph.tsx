/**
 * KnowledgeGraph 组件
 * 知识图谱可视化 - 展示记忆节点和关系
 */

import { useState, useEffect, useCallback, useRef } from 'react'
import { queryGraph, type GraphNode, type TimelineEvent } from '@/services/memory-api'
import { Network, GitBranch, Clock, ChevronRight, ZoomIn, ZoomOut, RotateCcw } from 'lucide-react'

interface GraphData {
  nodes: GraphNode[]
  edges: { source: string; target: string; type: string }[]
}

interface KnowledgeGraphProps {
  onNodeClick?: (nodeId: string) => void
}

export function KnowledgeGraph({ onNodeClick }: KnowledgeGraphProps) {
  const [graphData, setGraphData] = useState<GraphData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null)
  const [zoom, setZoom] = useState(1)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const [isDragging, setIsDragging] = useState(false)
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 })
  const containerRef = useRef<HTMLDivElement>(null)

  // 加载图数据
  const loadGraphData = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      // 获取所有记忆节点
      const nodesResponse = await queryGraph('timeline', { maxDepth: 3 })
      
      // 模拟边数据（基于时间顺序）
      const nodes = nodesResponse.nodes || []
      const edges: { source: string; target: string; type: string }[] = []
      
      // 构建时间顺序边
      for (let i = 0; i < nodes.length - 1; i++) {
        edges.push({
          source: nodes[i].id,
          target: nodes[i + 1].id,
          type: 'temporal'
        })
      }

      setGraphData({ nodes, edges })
    } catch (err) {
      console.error('Failed to load graph data:', err)
      setError('加载图数据失败，请确保后端服务运行中')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadGraphData()
  }, [loadGraphData])

  // 处理节点点击
  const handleNodeClick = (node: GraphNode) => {
    setSelectedNode(node)
    onNodeClick?.(node.id)
  }

  // 处理缩放
  const handleZoomIn = () => setZoom(z => Math.min(z + 0.2, 2))
  const handleZoomOut = () => setZoom(z => Math.max(z - 0.2, 0.4))
  const handleReset = () => {
    setZoom(1)
    setPan({ x: 0, y: 0 })
  }

  // 处理拖拽
  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.target === containerRef.current || (e.target as HTMLElement).classList.contains('graph-canvas')) {
      setIsDragging(true)
      setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y })
    }
  }

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      setPan({
        x: e.clientX - dragStart.x,
        y: e.clientY - dragStart.y
      })
    }
  }

  const handleMouseUp = () => {
    setIsDragging(false)
  }

  // 计算节点位置（简单的分层布局）
  const calculateNodePositions = () => {
    if (!graphData?.nodes.length) return {}

    const positions: Record<string, { x: number; y: number }> = {}
    const nodeCount = graphData.nodes.length
    const centerX = 400
    const centerY = 300
    const radius = Math.min(200, nodeCount * 30)

    graphData.nodes.forEach((node, index) => {
      const angle = (2 * Math.PI * index) / nodeCount - Math.PI / 2
      positions[node.id] = {
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle)
      }
    })

    return positions
  }

  const nodePositions = calculateNodePositions()

  // 获取节点颜色
  const getNodeColor = (type: string) => {
    switch (type) {
      case 'memory':
        return '#3b82f6' // blue
      case 'category':
        return '#10b981' // green
      case 'resource':
        return '#f59e0b' // amber
      case 'event':
        return '#8b5cf6' // purple
      default:
        return '#6b7280' // gray
    }
  }

  // 获取节点图标
  const getNodeIcon = (type: string) => {
    switch (type) {
      case 'memory':
        return <ChevronRight className="w-3 h-3" />
      case 'category':
        return <Network className="w-3 h-3" />
      case 'resource':
        return <Clock className="w-3 h-3" />
      default:
        return <GitBranch className="w-3 h-3" />
    }
  }

  if (loading) {
    return (
      <div className="knowledge-graph-loading">
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto mb-2"></div>
            <p className="text-sm text-gray-500">加载知识图谱...</p>
          </div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="knowledge-graph-error">
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <p className="text-red-500 mb-2">{error}</p>
            <button
              onClick={loadGraphData}
              className="px-3 py-1 bg-blue-500 text-white rounded text-sm hover:bg-blue-600"
            >
              重试
            </button>
          </div>
        </div>
      </div>
    )
  }

  if (!graphData?.nodes.length) {
    return (
      <div className="knowledge-graph-empty">
        <div className="flex items-center justify-center h-full">
          <div className="text-center text-gray-500">
            <Network className="w-12 h-12 mx-auto mb-2 opacity-50" />
            <p>暂无图数据</p>
            <p className="text-xs mt-1">请先创建一些记忆</p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="knowledge-graph">
      {/* 工具栏 */}
      <div className="graph-toolbar flex items-center gap-2 p-2 border-b border-gray-200 bg-gray-50">
        <button
          onClick={handleZoomIn}
          className="p-1.5 rounded hover:bg-gray-200"
          title="放大"
        >
          <ZoomIn className="w-4 h-4" />
        </button>
        <button
          onClick={handleZoomOut}
          className="p-1.5 rounded hover:bg-gray-200"
          title="缩小"
        >
          <ZoomOut className="w-4 h-4" />
        </button>
        <button
          onClick={handleReset}
          className="p-1.5 rounded hover:bg-gray-200"
          title="重置视图"
        >
          <RotateCcw className="w-4 h-4" />
        </button>
        <span className="ml-auto text-xs text-gray-500">
          {graphData.nodes.length} 节点 · {graphData.edges.length} 关系
        </span>
      </div>

      {/* 图画布 */}
      <div
        ref={containerRef}
        className="graph-canvas relative h-full overflow-hidden cursor-grab"
        style={{ cursor: isDragging ? 'grabbing' : 'grab' }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      >
        <svg
          className="absolute inset-0 w-full h-full"
          style={{
            transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
            transformOrigin: 'center center'
          }}
        >
          {/* 边 */}
          {graphData.edges.map((edge, index) => {
            const sourcePos = nodePositions[edge.source]
            const targetPos = nodePositions[edge.target]
            if (!sourcePos || !targetPos) return null

            return (
              <line
                key={`edge-${index}`}
                x1={sourcePos.x}
                y1={sourcePos.y}
                x2={targetPos.x}
                y2={targetPos.y}
                stroke="#d1d5db"
                strokeWidth={1.5}
                strokeDasharray={edge.type === 'temporal' ? '4,4' : '0'}
              />
            )
          })}

          {/* 节点 */}
          {graphData.nodes.map((node) => {
            const pos = nodePositions[node.id]
            if (!pos) return null

            const isSelected = selectedNode?.id === node.id

            return (
              <g
                key={node.id}
                transform={`translate(${pos.x}, ${pos.y})`}
                onClick={() => handleNodeClick(node)}
                style={{ cursor: 'pointer' }}
              >
                {/* 节点圆圈 */}
                <circle
                  r={isSelected ? 28 : 24}
                  fill={getNodeColor(node.type)}
                  stroke={isSelected ? '#1d4ed8' : 'transparent'}
                  strokeWidth={3}
                  className="transition-all duration-200"
                />
                {/* 节点图标 */}
                <foreignObject x={-12} y={-12} width={24} height={24}>
                  <div className="flex items-center justify-center w-full h-full text-white">
                    {getNodeIcon(node.type)}
                  </div>
                </foreignObject>
                {/* 节点标签 */}
                <text
                  y={40}
                  textAnchor="middle"
                  className="text-xs fill-gray-700"
                  style={{ fontSize: '11px' }}
                >
                  {node.label.length > 12 ? node.label.slice(0, 12) + '...' : node.label}
                </text>
              </g>
            )
          })}
        </svg>
      </div>

      {/* 节点详情面板 */}
      {selectedNode && (
        <div className="node-details absolute bottom-4 left-4 right-4 bg-white rounded-lg shadow-lg border border-gray-200 p-3">
          <div className="flex items-start justify-between">
            <div>
              <h3 className="font-medium text-gray-900">{selectedNode.label}</h3>
              <p className="text-xs text-gray-500 mt-0.5">
                类型: {selectedNode.type}
                {selectedNode.timestamp && ` · ${new Date(selectedNode.timestamp).toLocaleString('zh-CN')}`}
              </p>
            </div>
            <button
              onClick={() => setSelectedNode(null)}
              className="text-gray-400 hover:text-gray-600"
            >
              ×
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

export default KnowledgeGraph
