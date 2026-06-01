/**
 * Marketplace View - Plugin marketplace with browsing, search, and install
 */

import React, { useState, useEffect, useCallback } from 'react';
import { Search, Download, Star, Tag, Filter, X, ChevronLeft, ChevronRight } from 'lucide-react';
import { 
  listPlugins, 
  getTrending, 
  getFreePlugins,
  formatDownloads, 
  formatPrice,
  type Plugin,
  type PluginListParams
} from '@/services/marketplace-api';
import { LoadingSpinner, ErrorState, EmptyState } from '@/components/ui/loading';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';

const CATEGORIES = [
  { id: 'all', name: '全部', icon: '🔌' },
  { id: 'productivity', name: '效率工具', icon: '⚡' },
  { id: 'ai', name: 'AI & 机器学习', icon: '🤖' },
  { id: 'devtools', name: '开发者工具', icon: '🛠️' },
  { id: 'integrations', name: '集成', icon: '🔗' },
  { id: 'utilities', name: '实用工具', icon: '🔧' },
  { id: 'visual', name: '可视化', icon: '📊' },
];

const SORT_OPTIONS = [
  { value: 'downloads', label: '下载量' },
  { value: 'rating', label: '评分' },
  { value: 'recent', label: '最新' },
  { value: 'price', label: '价格' },
];

interface MarketplaceViewProps {
  onInstallPlugin?: (plugin: Plugin) => void;
}

export const MarketplaceView: React.FC<MarketplaceViewProps> = ({ onInstallPlugin }) => {
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [trending, setTrending] = useState<Plugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  // Search & Filter state
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [sortBy, setSortBy] = useState<'downloads' | 'rating' | 'recent' | 'price'>('downloads');
  const [tier, setTier] = useState<'all' | 'free' | 'paid'>('all');
  
  // Pagination
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [total, setTotal] = useState(0);

  const fetchPlugins = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params: PluginListParams = {
        category: selectedCategory !== 'all' ? selectedCategory : undefined,
        search: searchQuery || undefined,
        sort: sortBy,
        order: 'desc',
        page,
        limit: 12,
        tier,
      };
      
      const response = await listPlugins(params);
      setPlugins(response.plugins);
      setTotal(response.total);
      setTotalPages(response.total_pages);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载插件失败');
    } finally {
      setLoading(false);
    }
  }, [selectedCategory, searchQuery, sortBy, tier, page]);

  const fetchTrending = useCallback(async () => {
    try {
      const trendingPlugins = await getTrending(5);
      setTrending(trendingPlugins);
    } catch (err) {
      console.error('Failed to fetch trending:', err);
    }
  }, []);

  useEffect(() => {
    fetchPlugins();
  }, [fetchPlugins]);

  useEffect(() => {
    fetchTrending();
  }, [fetchTrending]);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    setPage(1);
    fetchPlugins();
  };

  const handleCategoryChange = (categoryId: string) => {
    setSelectedCategory(categoryId);
    setPage(1);
  };

  const handleInstall = (plugin: Plugin) => {
    if (onInstallPlugin) {
      onInstallPlugin(plugin);
    } else {
      // TODO: Call install API
      console.log('Install plugin:', plugin.id);
    }
  };

  if (error) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <ErrorState error={error} onRetry={fetchPlugins} />
      </div>
    );
  }

  return (
    <div className="marketplace-view h-full flex flex-col bg-background">
      {/* Header */}
      <div className="px-4 py-3 border-b">
        <h2 className="text-lg font-semibold mb-3">插件市场</h2>
        
        {/* Search */}
        <form onSubmit={handleSearch} className="relative mb-3">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            type="search"
            placeholder="搜索插件..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-10"
          />
          {searchQuery && (
            <button
              type="button"
              onClick={() => { setSearchQuery(''); setPage(1); }}
              className="absolute right-3 top-1/2 -translate-y-1/2"
            >
              <X className="h-4 w-4 text-muted-foreground hover:text-foreground" />
            </button>
          )}
        </form>

        {/* Filters */}
        <div className="flex gap-2 flex-wrap">
          {/* Category pills */}
          <div className="flex gap-1 flex-wrap">
            {CATEGORIES.map((cat) => (
              <button
                key={cat.id}
                onClick={() => handleCategoryChange(cat.id)}
                className={`px-2 py-1 text-xs rounded-full transition-colors ${
                  selectedCategory === cat.id
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted hover:bg-muted/80 text-muted-foreground'
                }`}
              >
                {cat.icon} {cat.name}
              </button>
            ))}
          </div>

          {/* Sort & Tier dropdowns */}
          <select
            value={sortBy}
            onChange={(e) => { setSortBy(e.target.value as typeof sortBy); setPage(1); }}
            className="px-2 py-1 text-xs rounded bg-muted border-0"
          >
            {SORT_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>{opt.label}</option>
            ))}
          </select>

          <select
            value={tier}
            onChange={(e) => { setTier(e.target.value as typeof tier); setPage(1); }}
            className="px-2 py-1 text-xs rounded bg-muted border-0"
          >
            <option value="all">全部价格</option>
            <option value="free">免费</option>
            <option value="paid">付费</option>
          </select>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center h-64">
            <LoadingSpinner size="lg" />
          </div>
        ) : plugins.length === 0 ? (
          <div className="flex items-center justify-center h-64">
            <EmptyState icon="search" title="没有找到插件" description="尝试调整搜索条件或浏览其他分类" />
          </div>
        ) : (
          <div className="p-4">
            {/* Results count */}
            <div className="text-sm text-muted-foreground mb-3">
              找到 {total} 个插件
            </div>

            {/* Plugin Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {plugins.map((plugin) => (
                <PluginCard 
                  key={plugin.id} 
                  plugin={plugin} 
                  onInstall={handleInstall}
                />
              ))}
            </div>

            {/* Pagination */}
            {totalPages > 1 && (
              <div className="flex items-center justify-center gap-2 mt-4 pt-4 border-t">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPage(p => Math.max(1, p - 1))}
                  disabled={page === 1}
                >
                  <ChevronLeft className="h-4 w-4" />
                </Button>
                <span className="text-sm text-muted-foreground">
                  第 {page} / {totalPages} 页
                </span>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                  disabled={page === totalPages}
                >
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

// ============ Plugin Card ============

interface PluginCardProps {
  plugin: Plugin;
  onInstall: (plugin: Plugin) => void;
}

function PluginCard({ plugin, onInstall }: PluginCardProps) {
  return (
    <div className="plugin-card border rounded-lg p-3 hover:border-primary/50 transition-colors">
      <div className="flex gap-3">
        {/* Thumbnail placeholder */}
        <div className="w-12 h-12 rounded bg-muted flex items-center justify-center text-xl shrink-0">
          {plugin.category.charAt(0).toUpperCase()}
        </div>
        
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <h3 className="font-medium truncate">{plugin.name}</h3>
              <p className="text-xs text-muted-foreground truncate">{plugin.description}</p>
            </div>
            <span className="text-sm font-medium shrink-0">
              {formatPrice(plugin.price_cents)}
            </span>
          </div>
          
          {/* Meta info */}
          <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Download className="h-3 w-3" />
              {formatDownloads(plugin.downloads)}
            </span>
            <span className="flex items-center gap-1">
              <Star className="h-3 w-3 fill-yellow-500 text-yellow-500" />
              {plugin.rating.toFixed(1)} ({plugin.rating_count})
            </span>
            {plugin.verified && (
              <span className="text-green-600">✓ 已验证</span>
            )}
          </div>
          
          {/* Tags */}
          {plugin.tags.length > 0 && (
            <div className="flex gap-1 mt-2 flex-wrap">
              {plugin.tags.slice(0, 3).map((tag) => (
                <span key={tag} className="px-1.5 py-0.5 text-[10px] rounded bg-muted">
                  {tag}
                </span>
              ))}
            </div>
          )}
          
          {/* Actions */}
          <div className="flex gap-2 mt-2">
            <Button size="sm" onClick={() => onInstall(plugin)}>
              <Download className="h-3 w-3 mr-1" />
              安装
            </Button>
            <Button size="sm" variant="outline">
              详情
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default MarketplaceView;
