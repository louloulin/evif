package evif

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// Client is a Go client for the EVIF REST API.
type Client struct {
	baseURL    string
	apiKey     string
	httpClient *http.Client
}

// NewClient creates a new EVIF client.
//
// baseURL should be the server root (e.g., "http://localhost:8080").
// The "/api/v1" prefix is appended automatically.
func NewClient(baseURL string) *Client {
	return &Client{
		baseURL: normalizeBaseURL(baseURL),
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// NewClientWithAPIKey creates a client with an API key for authentication.
func NewClientWithAPIKey(baseURL, apiKey string) *Client {
	c := NewClient(baseURL)
	c.apiKey = apiKey
	return c
}

// NewClientWithHTTPClient creates a client with a custom HTTP client.
func NewClientWithHTTPClient(baseURL string, httpClient *http.Client) *Client {
	return &Client{
		baseURL:    normalizeBaseURL(baseURL),
		httpClient: httpClient,
	}
}

func normalizeBaseURL(baseURL string) string {
	baseURL = strings.TrimRight(baseURL, "/")
	if !strings.Contains(baseURL, "://") {
		return baseURL
	}
	if !strings.HasSuffix(baseURL, "/api/v1") {
		baseURL += "/api/v1"
	}
	return baseURL
}

func (c *Client) doRequest(method, endpoint string, query url.Values, body io.Reader) (*http.Response, error) {
	u := c.baseURL + endpoint
	if len(query) > 0 {
		u += "?" + query.Encode()
	}

	req, err := http.NewRequest(method, u, body)
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}

	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	if c.apiKey != "" {
		req.Header.Set("Authorization", "Bearer "+c.apiKey)
	}

	return c.httpClient.Do(req)
}

func (c *Client) handleResponse(resp *http.Response, out any) error {
	defer resp.Body.Close()

	if resp.StatusCode >= 200 && resp.StatusCode < 300 {
		if out != nil {
			return json.NewDecoder(resp.Body).Decode(out)
		}
		return nil
	}

	var errResp APIResponse
	if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}
	if errResp.Error != "" {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, errResp.Error)
	}
	return fmt.Errorf("HTTP %d", resp.StatusCode)
}

func (c *Client) doJSON(method, endpoint string, query url.Values, body, out any) error {
	var bodyReader io.Reader
	if body != nil {
		data, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("marshal body: %w", err)
		}
		bodyReader = bytes.NewReader(data)
	}

	resp, err := c.doRequest(method, endpoint, query, bodyReader)
	if err != nil {
		return err
	}

	return c.handleResponse(resp, out)
}

// ── Health ───────────────────────────────────────────────────────

// Health checks the server health.
func (c *Client) Health() (*HealthResponse, error) {
	var result struct {
		Data HealthResponse `json:"data"`
	}
	if err := c.doJSON("GET", "/health", nil, nil, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// ── File Operations ──────────────────────────────────────────────

// ReadFile reads file content.
// offset and size: use -1 for defaults (read all from beginning).
func (c *Client) ReadFile(path string, offset, size int64) ([]byte, error) {
	query := url.Values{}
	query.Set("path", path)
	if offset > 0 {
		query.Set("offset", fmt.Sprintf("%d", offset))
	}
	if size >= 0 {
		query.Set("size", fmt.Sprintf("%d", size))
	}

	resp, err := c.doRequest("GET", "/files", query, nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		var errResp APIResponse
		_ = json.NewDecoder(resp.Body).Decode(&errResp)
		if errResp.Error != "" {
			return nil, fmt.Errorf("HTTP %d: %s", resp.StatusCode, errResp.Error)
		}
		return nil, fmt.Errorf("HTTP %d", resp.StatusCode)
	}

	return io.ReadAll(resp.Body)
}

// WriteFile writes data to a file.
func (c *Client) WriteFile(path string, data []byte) (*WriteResponse, error) {
	query := url.Values{}
	query.Set("path", path)

	resp, err := c.doRequest("PUT", "/files", query, bytes.NewReader(data))
	if err != nil {
		return nil, err
	}

	var result struct {
		Data WriteResponse `json:"data"`
	}
	if err := c.handleResponse(resp, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// CreateFile creates an empty file.
func (c *Client) CreateFile(path string) error {
	query := url.Values{}
	query.Set("path", path)
	return c.doJSON("POST", "/files", query, nil, nil)
}

// DeleteFile deletes a file or directory.
func (c *Client) DeleteFile(path string, recursive bool) error {
	query := url.Values{}
	query.Set("path", path)
	if recursive {
		query.Set("recursive", "true")
	}
	return c.doJSON("DELETE", "/files", query, nil, nil)
}

// ── Directory Operations ─────────────────────────────────────────

// ReadDir lists directory contents.
func (c *Client) ReadDir(path string) ([]FileInfo, error) {
	query := url.Values{}
	query.Set("path", path)

	var result struct {
		Data []FileInfo `json:"data"`
	}
	if err := c.doJSON("GET", "/directories", query, nil, &result); err != nil {
		return nil, err
	}
	return result.Data, nil
}

// Mkdir creates a directory.
func (c *Client) Mkdir(path string, parents bool) error {
	query := url.Values{}
	query.Set("path", path)
	if parents {
		query.Set("parents", "true")
	}
	return c.doJSON("POST", "/directories", query, nil, nil)
}

// DeleteDir deletes a directory.
func (c *Client) DeleteDir(path string, recursive bool) error {
	query := url.Values{}
	query.Set("path", path)
	if recursive {
		query.Set("recursive", "true")
	}
	return c.doJSON("DELETE", "/directories", query, nil, nil)
}

// ── Metadata Operations ──────────────────────────────────────────

// Stat returns file metadata.
func (c *Client) Stat(path string) (*StatResponse, error) {
	query := url.Values{}
	query.Set("path", path)

	var result struct {
		Data StatResponse `json:"data"`
	}
	if err := c.doJSON("GET", "/stat", query, nil, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// Rename renames or moves a file/directory.
func (c *Client) Rename(oldPath, newPath string) error {
	return c.doJSON("POST", "/rename", nil, RenameRequest{
		OldPath: oldPath,
		NewPath: newPath,
	}, nil)
}

// Touch updates file timestamps.
func (c *Client) Touch(path string) error {
	query := url.Values{}
	query.Set("path", path)
	return c.doJSON("POST", "/touch", query, nil, nil)
}

// Grep searches for a pattern in files.
func (c *Client) Grep(path, pattern string, recursive, caseInsensitive bool) (*GrepResponse, error) {
	var result struct {
		Data GrepResponse `json:"data"`
	}
	if err := c.doJSON("POST", "/grep", nil, GrepRequest{
		Path:            path,
		Pattern:         pattern,
		Recursive:       recursive,
		CaseInsensitive: caseInsensitive,
	}, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// ── Mount Operations ─────────────────────────────────────────────

// ListMounts returns all mount points.
func (c *Client) ListMounts() ([]MountInfo, error) {
	var result struct {
		Data []MountInfo `json:"data"`
	}
	if err := c.doJSON("GET", "/mounts", nil, nil, &result); err != nil {
		return nil, err
	}
	return result.Data, nil
}

// Mount mounts a plugin at the given path.
func (c *Client) Mount(pluginType, path string, config map[string]any) (*MountInfo, error) {
	var result struct {
		Data MountInfo `json:"data"`
	}
	if err := c.doJSON("POST", "/mount", nil, MountRequest{
		PluginType: pluginType,
		Path:       path,
		Config:     config,
	}, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// MountWithInstance mounts a plugin with a specific instance name.
func (c *Client) MountWithInstance(pluginType, path, instanceName string, config map[string]any) (*MountInfo, error) {
	var result struct {
		Data MountInfo `json:"data"`
	}
	if err := c.doJSON("POST", "/mount", nil, MountRequest{
		PluginType:   pluginType,
		Path:         path,
		InstanceName: instanceName,
		Config:       config,
	}, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// Unmount removes a mount point.
func (c *Client) Unmount(path string) error {
	return c.doJSON("POST", "/unmount", nil, map[string]string{"path": path}, nil)
}

// ── Plugin Operations ────────────────────────────────────────────

// ListPlugins returns all loaded plugins.
func (c *Client) ListPlugins() ([]PluginInfo, error) {
	var result struct {
		Data []PluginInfo `json:"data"`
	}
	if err := c.doJSON("GET", "/plugins", nil, nil, &result); err != nil {
		return nil, err
	}
	return result.Data, nil
}

// GetPluginStatus returns status for a specific plugin.
func (c *Client) GetPluginStatus(name string) (map[string]any, error) {
	var result struct {
		Data map[string]any `json:"data"`
	}
	if err := c.doJSON("GET", "/plugins/"+name+"/status", nil, nil, &result); err != nil {
		return nil, err
	}
	return result.Data, nil
}

// ── Handle Operations ────────────────────────────────────────────

// OpenHandle opens a stateful file handle.
func (c *Client) OpenHandle(path, mode string) (*HandleInfo, error) {
	var result struct {
		Data HandleInfo `json:"data"`
	}
	if err := c.doJSON("POST", "/handles/open", nil, HandleOpenRequest{
		Path: path,
		Mode: mode,
	}, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// ReadHandle reads from an open handle.
func (c *Client) ReadHandle(handleID string, size int) (*HandleReadResult, error) {
	var result struct {
		Data HandleReadResult `json:"data"`
	}
	if err := c.doJSON("POST", fmt.Sprintf("/handles/%s/read", handleID), nil,
		map[string]int{"size": size}, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// WriteHandle writes to an open handle.
func (c *Client) WriteHandle(handleID string, data []byte) (*HandleWriteResult, error) {
	var result struct {
		Data HandleWriteResult `json:"data"`
	}
	if err := c.doJSON("POST", fmt.Sprintf("/handles/%s/write", handleID), nil,
		map[string]string{"content": string(data)}, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// SeekHandle seeks to a position in a handle.
func (c *Client) SeekHandle(handleID string, offset int64) (*HandleSeekResult, error) {
	var result struct {
		Data HandleSeekResult `json:"data"`
	}
	if err := c.doJSON("POST", fmt.Sprintf("/handles/%s/seek", handleID), nil,
		map[string]int64{"offset": offset}, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// CloseHandle closes an open handle.
func (c *Client) CloseHandle(handleID string) error {
	return c.doJSON("POST", fmt.Sprintf("/handles/%s/close", handleID), nil, nil, nil)
}

// ListHandles returns all open handles.
func (c *Client) ListHandles() ([]HandleInfo, error) {
	var result struct {
		Data []HandleInfo `json:"data"`
	}
	if err := c.doJSON("GET", "/handles", nil, nil, &result); err != nil {
		return nil, err
	}
	return result.Data, nil
}

// ── Memory Operations ────────────────────────────────────────────

// CreateMemory creates a new memory.
func (c *Client) CreateMemory(req MemoryCreateRequest) (*Memory, error) {
	var result struct {
		Data Memory `json:"data"`
	}
	if err := c.doJSON("POST", "/memories", nil, req, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// GetMemory retrieves a memory by ID.
func (c *Client) GetMemory(memoryID string) (*Memory, error) {
	var result struct {
		Data Memory `json:"data"`
	}
	if err := c.doJSON("GET", "/memories/"+memoryID, nil, nil, &result); err != nil {
		return nil, err
	}
	return &result.Data, nil
}

// ListMemories lists memories with pagination.
func (c *Client) ListMemories(limit, offset int) ([]Memory, error) {
	query := url.Values{}
	query.Set("limit", fmt.Sprintf("%d", limit))
	query.Set("offset", fmt.Sprintf("%d", offset))

	var result struct {
		Data []Memory `json:"data"`
	}
	if err := c.doJSON("GET", "/memories", query, nil, &result); err != nil {
		return nil, err
	}
	return result.Data, nil
}

// SearchMemories searches memories by semantic similarity.
func (c *Client) SearchMemories(req MemorySearchRequest) ([]MemorySearchResult, error) {
	var result struct {
		Data []MemorySearchResult `json:"data"`
	}
	if err := c.doJSON("POST", "/memories/search", nil, req, &result); err != nil {
		return nil, err
	}
	return result.Data, nil
}

// DeleteMemory deletes a memory.
func (c *Client) DeleteMemory(memoryID string) error {
	return c.doJSON("DELETE", "/memories/"+memoryID, nil, nil, nil)
}
