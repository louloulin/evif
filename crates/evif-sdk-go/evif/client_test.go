package evif

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// ── Helpers ──────────────────────────────────────────────────────

func newTestServer(handler http.HandlerFunc) *httptest.Server {
	return httptest.NewServer(handler)
}

func apiDataResponse(data any) map[string]any {
	return map[string]any{"data": data, "status": "ok"}
}

// ── Client Construction ──────────────────────────────────────────

func TestNewClient_NormalizesBaseURL(t *testing.T) {
	c := NewClient("http://localhost:8080")
	assert.Equal(t, "http://localhost:8080/api/v1", c.baseURL)

	c2 := NewClient("http://localhost:8080/api/v1")
	assert.Equal(t, "http://localhost:8080/api/v1", c2.baseURL)

	c3 := NewClient("http://localhost:8080/")
	assert.Equal(t, "http://localhost:8080/api/v1", c3.baseURL)
}

func TestNewClientWithAPIKey(t *testing.T) {
	c := NewClientWithAPIKey("http://localhost:8080", "sk-test")
	assert.Equal(t, "sk-test", c.apiKey)
}

// ── Health ───────────────────────────────────────────────────────

func TestHealth(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/health", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse(map[string]string{
			"status":  "ok",
			"version": "0.1.0",
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	health, err := c.Health()
	require.NoError(t, err)
	assert.Equal(t, "ok", health.Status)
	assert.Equal(t, "0.1.0", health.Version)
}

// ── File Operations ──────────────────────────────────────────────

func TestReadFile(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/files", r.URL.Path)
		assert.Equal(t, "GET", r.Method)
		assert.Equal(t, "/memfs/test.txt", r.URL.Query().Get("path"))
		w.Write([]byte("hello world"))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	data, err := c.ReadFile("/memfs/test.txt", -1, -1)
	require.NoError(t, err)
	assert.Equal(t, []byte("hello world"), data)
}

func TestWriteFile(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/files", r.URL.Path)
		assert.Equal(t, "PUT", r.Method)
		assert.Equal(t, "/memfs/test.txt", r.URL.Query().Get("path"))
		json.NewEncoder(w).Encode(apiDataResponse(map[string]any{
			"bytes_written": 5,
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	resp, err := c.WriteFile("/memfs/test.txt", []byte("hello"))
	require.NoError(t, err)
	assert.Equal(t, int64(5), resp.BytesWritten)
}

func TestCreateFile(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "POST", r.Method)
		assert.Equal(t, "/memfs/new.txt", r.URL.Query().Get("path"))
		json.NewEncoder(w).Encode(apiDataResponse(nil))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.CreateFile("/memfs/new.txt")
	require.NoError(t, err)
}

func TestDeleteFile(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "DELETE", r.Method)
		json.NewEncoder(w).Encode(apiDataResponse(nil))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.DeleteFile("/memfs/test.txt", false)
	require.NoError(t, err)
}

// ── Directory Operations ─────────────────────────────────────────

func TestReadDir(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/directories", r.URL.Path)
		assert.Equal(t, "GET", r.Method)
		json.NewEncoder(w).Encode(apiDataResponse([]map[string]any{
			{"name": "file1.txt", "size": 100, "is_dir": false},
			{"name": "subdir", "size": 0, "is_dir": true},
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	entries, err := c.ReadDir("/memfs")
	require.NoError(t, err)
	require.Len(t, entries, 2)
	assert.Equal(t, "file1.txt", entries[0].Name)
	assert.False(t, entries[0].IsDir)
	assert.True(t, entries[1].IsDir)
}

func TestMkdir(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "POST", r.Method)
		json.NewEncoder(w).Encode(apiDataResponse(nil))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.Mkdir("/memfs/newdir", true)
	require.NoError(t, err)
}

// ── Stat ─────────────────────────────────────────────────────────

func TestStat(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/stat", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse(map[string]any{
			"name": "test.txt", "size": 42, "mode": 420, "is_dir": false,
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	stat, err := c.Stat("/memfs/test.txt")
	require.NoError(t, err)
	assert.Equal(t, "test.txt", stat.Name)
	assert.Equal(t, int64(42), stat.Size)
}

// ── Rename ───────────────────────────────────────────────────────

func TestRename(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/rename", r.URL.Path)
		assert.Equal(t, "POST", r.Method)
		var req RenameRequest
		json.NewDecoder(r.Body).Decode(&req)
		assert.Equal(t, "/memfs/a.txt", req.OldPath)
		assert.Equal(t, "/memfs/b.txt", req.NewPath)
		json.NewEncoder(w).Encode(apiDataResponse(nil))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.Rename("/memfs/a.txt", "/memfs/b.txt")
	require.NoError(t, err)
}

// ── Grep ─────────────────────────────────────────────────────────

func TestGrep(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/grep", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse(map[string]any{
			"matches": []map[string]any{
				{"file": "a.txt", "line": 1, "content": "match"},
			},
			"count": 1,
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	result, err := c.Grep("/memfs", "pattern", true, false)
	require.NoError(t, err)
	assert.Equal(t, 1, result.Count)
	require.Len(t, result.Matches, 1)
	assert.Equal(t, "a.txt", result.Matches[0].File)
}

// ── Mount Operations ─────────────────────────────────────────────

func TestListMounts(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/mounts", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse([]map[string]any{
			{"path": "/memfs", "plugin_type": "memfs"},
			{"path": "/s3/aws", "plugin_type": "s3fs", "instance_name": "aws"},
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	mounts, err := c.ListMounts()
	require.NoError(t, err)
	require.Len(t, mounts, 2)
	assert.Equal(t, "/memfs", mounts[0].Path)
	assert.Equal(t, "aws", mounts[1].InstanceName)
}

func TestMount(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/mount", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse(map[string]any{
			"path": "/s3/aws", "plugin_type": "s3fs",
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	info, err := c.Mount("s3fs", "/s3/aws", map[string]any{"region": "us-west-1"})
	require.NoError(t, err)
	assert.Equal(t, "/s3/aws", info.Path)
}

func TestUnmount(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/unmount", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse(nil))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.Unmount("/s3/aws")
	require.NoError(t, err)
}

// ── Plugin Operations ────────────────────────────────────────────

func TestListPlugins(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/plugins", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse([]map[string]any{
			{"name": "memfs", "mount_path": "/memfs"},
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	plugins, err := c.ListPlugins()
	require.NoError(t, err)
	require.Len(t, plugins, 1)
	assert.Equal(t, "memfs", plugins[0].Name)
}

// ── Handle Operations ────────────────────────────────────────────

func TestOpenHandle(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/handles/open", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse(map[string]any{
			"id": "h-1", "path": "/memfs/file.txt", "mode": "read", "offset": 0,
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	handle, err := c.OpenHandle("/memfs/file.txt", "read")
	require.NoError(t, err)
	assert.Equal(t, "h-1", handle.ID)
	assert.Equal(t, "/memfs/file.txt", handle.Path)
}

func TestReadHandle(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Contains(t, r.URL.Path, "/handles/h-1/read")
		json.NewEncoder(w).Encode(apiDataResponse(map[string]any{
			"content": "hello", "size": 5,
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	result, err := c.ReadHandle("h-1", 5)
	require.NoError(t, err)
	assert.Equal(t, "hello", result.Content)
}

func TestCloseHandle(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Contains(t, r.URL.Path, "/handles/h-1/close")
		json.NewEncoder(w).Encode(apiDataResponse(nil))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.CloseHandle("h-1")
	require.NoError(t, err)
}

// ── Memory Operations ────────────────────────────────────────────

func TestCreateMemory(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/memories", r.URL.Path)
		assert.Equal(t, "POST", r.Method)
		var req MemoryCreateRequest
		json.NewDecoder(r.Body).Decode(&req)
		assert.Equal(t, "test content", req.Content)

		json.NewEncoder(w).Encode(apiDataResponse(map[string]any{
			"id": "mem-1", "content": "test content",
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	mem, err := c.CreateMemory(MemoryCreateRequest{Content: "test content"})
	require.NoError(t, err)
	assert.Equal(t, "mem-1", mem.ID)
}

func TestSearchMemories(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/memories/search", r.URL.Path)
		json.NewEncoder(w).Encode(apiDataResponse([]map[string]any{
			{"memory": map[string]any{"id": "mem-1", "content": "note"}, "score": 0.95},
		}))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	results, err := c.SearchMemories(MemorySearchRequest{Query: "note", K: 5})
	require.NoError(t, err)
	require.Len(t, results, 1)
	assert.Equal(t, "mem-1", results[0].Memory.ID)
}

func TestDeleteMemory(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "/api/v1/memories/mem-1", r.URL.Path)
		assert.Equal(t, "DELETE", r.Method)
		json.NewEncoder(w).Encode(apiDataResponse(nil))
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	err := c.DeleteMemory("mem-1")
	require.NoError(t, err)
}

// ── Error Handling ───────────────────────────────────────────────

func TestErrorResponse(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(map[string]string{
			"error": "file not found",
		})
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	_, err := c.Stat("/nonexistent")
	require.Error(t, err)
	assert.Contains(t, err.Error(), "file not found")
}

func TestServerError(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprint(w, "internal error")
	})
	defer srv.Close()

	c := NewClient(srv.URL)
	_, err := c.Stat("/test")
	require.Error(t, err)
	assert.Contains(t, err.Error(), "500")
}

// ── Authentication ───────────────────────────────────────────────

func TestAPIKeySent(t *testing.T) {
	srv := newTestServer(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal(t, "Bearer sk-test-key", r.Header.Get("Authorization"))
		json.NewEncoder(w).Encode(apiDataResponse(nil))
	})
	defer srv.Close()

	c := NewClientWithAPIKey(srv.URL, "sk-test-key")
	_, err := c.ListMounts()
	require.NoError(t, err)
}
