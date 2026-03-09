/**
 * KnowledgeGraph 组件
 * 知识图谱可视化 - 展示记忆节点和关系
 * 支持图查询 UI: 因果链、时间线、时序 BFS、时序路径
 */

import { useState, useEffect, useCallback, useRef } from 'react'
import { queryGraph, type GraphNode, type TimelineEvent, type GraphQueryResponse } from '@/services/memory-api'
import { Network, GitBranch, Clock, ChevronRight, ZoomIn, ZoomOut, RotateCcw, Search, Play, X } from 'lucide-react'

// 查询类型
type QueryType = 'timeline' | 'causal_chain' | 'temporal_bfs' | 'temporal_path'

interface QueryParams {
  startNode: string
  endNode: string
  maxDepth: number
  eventType: string
  startTime: string
  endTime: string
}

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

  // 图查询 UI 状态
  const [showQueryPanel, setShowQueryPanel] = useState(false)
  const [queryType, setQueryType] = useState<QueryType>('timeline')
  const [queryParams, setQueryParams] = useState<QueryParams>({
    startNode: '',
    endNode: '',
    maxDepth: 3,
    eventType: '',
    startTime: '',
    endTime: ''
  })
  const [queryResult, setQueryResult] = useState<GraphQueryResponse | null>(null)
  const [queryLoading, setQueryLoading] = useState(false)

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

  // 执行图查询
  const executeQuery = useCallback(async () => {
    setQueryLoading(true)
    setError(null)
    try {
      const result = await queryGraph(queryType, {
        startNode: queryParams.startNode || undefined,
        endNode: queryParams.endNode || undefined,
        maxDepth: queryParams.maxDepth,
        eventType: queryParams.eventType || undefined,
        startTime: queryParams.startTime || undefined,
        endTime: queryParams.endTime || undefined
      })
      setQueryResult(result)

      // 如果是 timeline 查询，更新图数据
      if (queryType === 'timeline' && result.timeline) {
        const nodes = result.nodes || []
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
      }
    } catch (err) {
      console.error('Failed to execute query:', err)
      setError('查询执行失败，请检查参数')
    } finally {
      setQueryLoading(false)
    }
  }, [queryType, queryParams])

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
        <div className="w-px h-5 bg-gray-300 mx-1"></div>
        <button
          onClick={() => setShowQueryPanel(!showQueryPanel)}
          className={`p-1.5 rounded ${showQueryPanel ? 'bg-blue-100 text-blue-600' : 'hover:bg-gray-200'}`}
          title="图查询"
        >
          <Search className="w-4 h-4" />
        </button>
        <span className="ml-auto text-xs text-gray-500">
          {graphData.nodes.length} 节点 · {graphData.edges.length} 关系
        </span>
      </div>

      {/* 图查询面板 */}
      {showQueryPanel && (
        <div className="query-panel absolute top-12 left-2 right-2 bg-white rounded-lg shadow-lg border border-gray-200 p-3 z-10">
          <div className="flex items-center justify-between mb-3">
            <h3 className="font-medium text-gray-900 flex items-center gap-2">
              <Search className="w-4 h-4" />
              图查询
            </h3>
            <button
              onClick={() => setShowQueryPanel(false)}
              className="text-gray-400 hover:text-gray-600"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* 查询类型选择 */}
          <div className="mb-3">
            <label className="block text-xs text-gray-600 mb-1">查询类型</label>
            <div className="flex flex-wrap gap-1">
              {[
                { value: 'timeline', label: '时间线' },
                { value: 'causal_chain', label: '因果链' },
                { value: 'temporal_bfs', label: '时序 BFS' },
                { value: 'temporal_path', label: '时序路径' }
              ].map((type) => (
                <button
                  key={type.value}
                  onClick={() => setQueryType(type.value as QueryType)}
                  className={`px-2 py-1 text-xs rounded ${
                    queryType === type.value
                      ? 'bg-blue-500 text-white'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  {type.label}
                </button>
              ))}
            </div>
          </div>

          {/* 参数输入 */}
          <div className="grid grid-cols-2 gap-2 mb-3">
            <div>
              <label className="block text-xs text-gray-600 mb-1">起始节点 ID</label>
              <input
                type="text"
                value={queryParams.startNode}
                onChange={(e) => setQueryParams({ ...queryParams, startNode: e.target.value })}
                placeholder="可选"
                className="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-600 mb-1">目标节点 ID</label>
              <input
                type="text"
                value={queryParams.endNode}
                onChange={(e) => setQueryParams({ ...queryParams, endNode: e.target.value })}
                placeholder="可选"
                className="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-600 mb-1">最大深度</label>
              <input
                type="number"
                value={queryParams.maxDepth}
                onChange={(e) => setQueryParams({ ...queryParams, maxDepth: parseInt(e.target.value) || 3 })}
                min={1}
                max={10}
                className="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-600 mb-1">事件类型</label>
              <input
                type="text"
                value={queryParams.eventType}
                onChange={(e) => setQueryParams({ ...queryParams, eventType: e.target.value })}
                placeholder="可选"
                className="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-600 mb-1">开始时间</label>
              <input
                type="datetime-local"
                value={queryParams.startTime}
                onChange={(e) => setQueryParams({ ...queryParams, startTime: e.target.value })}
                className="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-600 mb-1">结束时间</label>
              <input
                type="datetime-local"
                value={queryParams.endTime}
                onChange={(e) => setQueryParams({ ...queryParams, endTime: e.target.value })}
                className="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:border-blue-500"
              />
            </div>
          </div>

          {/* 执行按钮 */}
          <button
            onClick={executeQuery}
            disabled={queryLoading}
            className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
          >
            {queryLoading ? (
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
            ) : (
              <Play className="w-4 h-4" />
            )}
            执行查询
          </button>

          {/* 查询结果 */}
          {queryResult && (
            <div className="mt-3 pt-3 border-t border-gray-200">
              <h4 className="text-sm font-medium text-gray-700 mb-2">
                查询结果 ({queryResult.total} 条)
              </h4>
              {queryResult.nodes && queryResult.nodes.length > 0 && (
                <div className="max-h-40 overflow-y-auto">
                  <table className="w-full text-xs">
                    <thead>
                      <tr className="text-left text-gray-500">
                        <th className="pb-1">ID</th>
                        <th className="pb-1">类型</th>
                        <th className="pb-1">标签</th>
                      </tr>
                    </thead>
                    <tbody>
                      {queryResult.nodes.slice(0, 10).map((node) => (
                        <tr key={node.id} className="border-t border-gray-100">
                          <td className="py-1 text-gray-600 truncate max-w-[100px]">{node.id.slice(0, 8)}...</td>
                          <td className="py-1">
                            <span className="px-1 py-0.5 bg-blue-100 text-blue-700 rounded text-xs">
                              {node.type}
                            </span>
                          </td>
                          <td className="py-1 truncate max-w-[120px]">{node.label}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {queryResult.nodes.length > 10 && (
                    <p className="text-xs text-gray-500 mt-1">...还有 {queryResult.nodes.length - 10} 条</p>
                  )}
                </div>
              )}
              {queryResult.timeline && queryResult.timeline.length > 0 && (
                <div className="max-h-40 overflow-y-auto">
                  <div className="space-y-1">
                    {queryResult.timeline.slice(0, 5).map((event, idx) => (
                      <div key={idx} className="flex items-center gap-2 text-xs p-1 bg-gray-50 rounded">
                        <span className="px-1 py-0.5 bg-purple-100 text-purple-700 rounded">
                          {event.event_type}
                        </span>
                        <span className="text-gray-600 truncate flex-1">
                          {event.node_id.slice(0, 12)}...
                        </span>
                        <span className="text-gray-400">
                          {new Date(event.timestamp).toLocaleString('zh-CN')}
                        </span>
                      </div>
                    ))}
                  </div>
                  {queryResult.timeline.length > 5 && (
                    <p className="text-xs text-gray-500 mt-1">...还有 {queryResult.timeline.length - 5} 条</p>
                  )}
                </div>
              )}
              {((!queryResult.nodes || queryResult.nodes.length === 0) &&
                (!queryResult.timeline || queryResult.timeline.length === 0)) && (
                <p className="text-xs text-gray-500">无查询结果</p>
              )}
            </div>
          )}
        </div>
      )}

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
