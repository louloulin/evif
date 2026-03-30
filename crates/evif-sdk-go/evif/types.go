package evif

import "time"

// FileInfo represents file or directory metadata.
type FileInfo struct {
	Name    string `json:"name"`
	Size    int64  `json:"size"`
	Mode    uint32 `json:"mode"`
	ModTime string `json:"mod_time"`
	IsDir   bool   `json:"is_dir"`
	Path    string `json:"path,omitempty"`
}

// ListResponse is the response from a directory listing.
type ListResponse struct {
	Entries []FileInfo `json:"entries"`
}

// ReadResponse is the response from a file read.
type ReadResponse struct {
	Content  string `json:"content"`
	Encoding string `json:"encoding,omitempty"`
	Size     int64  `json:"size"`
}

// WriteResponse is the response from a file write.
type WriteResponse struct {
	BytesWritten int64 `json:"bytes_written"`
}

// StatResponse is the response from a stat query.
type StatResponse struct {
	Name    string `json:"name"`
	Size    int64  `json:"size"`
	Mode    uint32 `json:"mode"`
	ModTime string `json:"mod_time"`
	IsDir   bool   `json:"is_dir"`
	Nlink   uint64 `json:"nlink,omitempty"`
	UID     uint32 `json:"uid,omitempty"`
	GID     uint32 `json:"gid,omitempty"`
}

// MountInfo represents a mount point.
type MountInfo struct {
	Path         string         `json:"path"`
	PluginType   string         `json:"plugin_type"`
	InstanceName string         `json:"instance_name,omitempty"`
	Config       map[string]any `json:"config,omitempty"`
	CreatedAt    string         `json:"created_at,omitempty"`
}

// PluginInfo represents a loaded plugin.
type PluginInfo struct {
	Name        string         `json:"name"`
	Description string         `json:"description,omitempty"`
	Version     string         `json:"version,omitempty"`
	MountPath   string         `json:"mount_path,omitempty"`
	Config      map[string]any `json:"config,omitempty"`
}

// HandleInfo represents an open file handle.
type HandleInfo struct {
	ID     string `json:"id"`
	Path   string `json:"path"`
	Mode   string `json:"mode"`
	Offset int64  `json:"offset"`
	TTL    int    `json:"ttl,omitempty"`
}

// HandleOpenRequest is the request body for opening a handle.
type HandleOpenRequest struct {
	Path string `json:"path"`
	Mode string `json:"mode"` // "read", "write", "append"
}

// HandleReadResult is the result from a handle read.
type HandleReadResult struct {
	Content string `json:"content"`
	Size    int    `json:"size"`
}

// HandleWriteResult is the result from a handle write.
type HandleWriteResult struct {
	BytesWritten int `json:"bytes_written"`
}

// HandleSeekResult is the result from a handle seek.
type HandleSeekResult struct {
	Offset int64 `json:"offset"`
}

// GrepRequest is the request body for grep search.
type GrepRequest struct {
	Path            string `json:"path"`
	Pattern         string `json:"pattern"`
	Recursive       bool   `json:"recursive"`
	CaseInsensitive bool   `json:"case_insensitive,omitempty"`
}

// GrepMatch represents a single grep match.
type GrepMatch struct {
	File    string `json:"file"`
	Line    int    `json:"line"`
	Content string `json:"content"`
}

// GrepResponse is the response from a grep search.
type GrepResponse struct {
	Matches []GrepMatch `json:"matches"`
	Count   int         `json:"count"`
}

// RenameRequest is the request body for rename/move.
type RenameRequest struct {
	OldPath string `json:"old_path"`
	NewPath string `json:"new_path"`
}

// MountRequest is the request body for mounting a plugin.
type MountRequest struct {
	PluginType   string         `json:"plugin_type"`
	Path         string         `json:"path"`
	InstanceName string         `json:"instance_name,omitempty"`
	Config       map[string]any `json:"config,omitempty"`
}

// BatchCopyRequest is the request body for batch copy.
type BatchCopyRequest struct {
	Sources []string `json:"sources"`
	Dest    string   `json:"dest"`
}

// BatchDeleteRequest is the request body for batch delete.
type BatchDeleteRequest struct {
	Paths []string `json:"paths"`
}

// Memory represents a memory item.
type Memory struct {
	ID          string            `json:"id"`
	Content     string            `json:"content"`
	Summary     string            `json:"summary,omitempty"`
	MemoryType  string            `json:"memory_type,omitempty"`
	Tags        []string          `json:"tags,omitempty"`
	Modality    string            `json:"modality,omitempty"`
	Metadata    map[string]any    `json:"metadata,omitempty"`
	CreatedAt   time.Time         `json:"created_at,omitempty"`
	UpdatedAt   time.Time         `json:"updated_at,omitempty"`
}

// MemoryCreateRequest is the request body for creating a memory.
type MemoryCreateRequest struct {
	Content    string         `json:"content"`
	MemoryType string         `json:"memory_type,omitempty"`
	Tags       []string       `json:"tags,omitempty"`
	Modality   string         `json:"modality,omitempty"`
	Metadata   map[string]any `json:"metadata,omitempty"`
}

// MemorySearchRequest is the request body for searching memories.
type MemorySearchRequest struct {
	Query     string  `json:"query"`
	K         int     `json:"k,omitempty"`
	Threshold float64 `json:"threshold,omitempty"`
	Mode      string  `json:"mode,omitempty"`
}

// MemorySearchResult represents a memory search result.
type MemorySearchResult struct {
	Memory Memory  `json:"memory"`
	Score  float64 `json:"score"`
}

// HealthResponse is the response from a health check.
type HealthResponse struct {
	Status  string `json:"status"`
	Version string `json:"version,omitempty"`
	Uptime  string `json:"uptime,omitempty"`
}

// APIResponse wraps the standard EVIF API response.
type APIResponse struct {
	Data   any    `json:"data,omitempty"`
	Error  string `json:"error,omitempty"`
	Status string `json:"status,omitempty"`
}
