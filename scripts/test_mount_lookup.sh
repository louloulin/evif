#!/bin/bash

echo "Testing mount point lookup..."
echo ""

echo "1. Root path:"
curl -s 'http://localhost:8081/api/v1/fs/list?path=/' | jq .
echo ""

echo "2. /hello mount point:"
curl -s 'http://localhost:8081/api/v1/fs/list?path=/hello' | jq .
echo ""

echo "3. /mem mount point:"
curl -s 'http://localhost:8081/api/v1/fs/list?path=/mem' | jq .
echo ""

echo "4. /local mount point:"
curl -s 'http://localhost:8081/api/v1/fs/list?path=/local' | jq .
echo ""

echo "5. Available mounts:"
curl -s 'http://localhost:8081/api/v1/mounts' | jq .
