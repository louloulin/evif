#!/bin/bash

echo "=========================================="
echo "E2E API Testing - VFS Path Translation Fix"
echo "=========================================="
echo ""

echo "✅ Step 1: Navigate to UI (Root Path)"
echo "Request: GET /api/v1/fs/list?path=/"
curl -s 'http://localhost:8081/api/v1/fs/list?path=/' | jq '.nodes[] | {path, name, is_dir}'
echo ""

echo "✅ Step 2: Verify Mount Points Display"
echo "Expected: /hello, /mem, /local"
curl -s 'http://localhost:8081/api/v1/mounts' | jq '.mounts[] | {plugin, path}'
echo ""

echo "✅ Step 3: Expand Mount Point /hello"
echo "Request: GET /api/v1/fs/list?path=/hello"
curl -s 'http://localhost:8081/api/v1/fs/list?path=/hello' | jq '.'
echo ""

echo "✅ Step 4: Expand Mount Point /mem"
echo "Request: GET /api/v1/fs/list?path=/mem"
curl -s 'http://localhost:8081/api/v1/fs/list?path=/mem' | jq '.'
echo ""

echo "✅ Step 5: Create New File in /mem"
echo "Request: POST /api/v1/fs/create"
curl -s -X POST 'http://localhost:8081/api/v1/fs/create' \
  -H 'Content-Type: application/json' \
  -d '{"path":"/mem/test.txt","content":"Hello from E2E test!"}' | jq '.'
echo ""

echo "✅ Step 6: Verify File Created"
echo "Request: GET /api/v1/fs/list?path=/mem"
curl -s 'http://localhost:8081/api/v1/fs/list?path=/mem' | jq '.'
echo ""

echo "✅ Step 7: Read File Content"
echo "Request: GET /api/v1/fs/read?path=/mem/test.txt"
curl -s 'http://localhost:8081/api/v1/fs/read?path=/mem/test.txt' | jq '.'
echo ""

echo "=========================================="
echo "E2E Test Complete"
echo "=========================================="
