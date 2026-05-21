-- evif-verify-mvp80.scpt - EVIF MVP 8.0 性能验证
-- 使用 osascript 运行，验证 MVP 8.0 生产就绪功能
--
-- 验证项目:
-- 1. 性能优化 (缓存层、连接池)
-- 2. 监控增强 (Prometheus metrics)
-- 3. MCP HTTP SSE (可选)
-- 4. 认证增强 (Token)

property REST_URL : "http://localhost:8080"
property METRICS_URL : "http://localhost:8080/metrics"
property MCP_URL : "http://localhost:8080/api/v1/mcp"

-- 颜色代码
property GREEN : "✅"
property RED : "❌"
property YELLOW : "⚠️"

on run argv
    set output to ""

    set output to output & lineBreak()
    set output to output & "==========================================" & lineBreak()
    set output to output & "EVIF MVP 8.0 Production Readiness" & lineBreak()
    set output to output & "Date: " & (current date as string) & lineBreak()
    set output to output & "==========================================" & lineBreak()
    set output to output & lineBreak()

    -- Step 1: Server Baseline
    set output to output & "[Step 1] Server Baseline" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkServerBaseline() & lineBreak()

    -- Step 2: Performance Metrics
    set output to output & lineBreak() & "[Step 2] Performance Metrics" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkPerformanceMetrics() & lineBreak()

    -- Step 3: Prometheus Metrics
    set output to output & lineBreak() & "[Step 3] Prometheus Metrics" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkPrometheusMetrics() & lineBreak()

    -- Step 4: MCP HTTP Features
    set output to output & lineBreak() & "[Step 4] MCP HTTP Features" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkMCPHttpFeatures() & lineBreak()

    -- Step 5: Token Optimization
    set output to output & lineBreak() & "[Step 5] Token Optimization" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkTokenOptimization() & lineBreak()

    -- Step 6: SSE Readiness (if implemented)
    set output to output & lineBreak() & "[Step 6] SSE Readiness" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkSSEReadiness() & lineBreak()

    -- Step 7: Authentication (if implemented)
    set output to output & lineBreak() & "[Step 7] Authentication" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkAuthentication() & lineBreak()

    -- Step 8: Health Deep Check
    set output to output & lineBreak() & "[Step 8] Health Deep Check" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkHealthDeep() & lineBreak()

    -- Summary
    set output to output & lineBreak() & "==========================================" & lineBreak()
    set output to output & "MVP 8.0 Verification Summary" & lineBreak()
    set output to output & "==========================================" & lineBreak()
    set output to output & lineBreak()
    set output to output & "All checks completed at " & (current date as string) & lineBreak()

    return output
end run

-- Step 1: Server Baseline
on checkServerBaseline()
    set output to ""

    -- REST Health
    set healthCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/health' 2>/dev/null"
    set health to do shell script healthCmd
    if health contains "healthy" then
        set output to output & GREEN & " REST Server: healthy" & lineBreak()
    else
        set output to output & RED & " REST Server: FAIL" & lineBreak()
    end if

    -- MCP Health
    set mcpHealthCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/mcp/health' 2>/dev/null"
    set mcpHealth to do shell script mcpHealthCmd
    if mcpHealth contains "healthy" then
        set output to output & GREEN & " MCP Server: healthy" & lineBreak()
    else
        set output to output & YELLOW & " MCP Server: not available" & lineBreak()
    end if

    -- Metrics Endpoint
    set metricsCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/metrics' 2>/dev/null"
    set metrics to do shell script metricsCmd
    if length of metrics > 100 then
        set output to output & GREEN & " Metrics endpoint: accessible" & lineBreak()
    else
        set output to output & YELLOW & " Metrics endpoint: limited" & lineBreak()
    end if

    return output
end checkServerBaseline

-- Step 2: Performance Metrics
on checkPerformanceMetrics()
    set output to ""

    -- Check if metrics include performance indicators
    set metricsCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/metrics' 2>/dev/null"
    set metrics to do shell script metricsCmd

    -- Check for cache-related metrics
    if metrics contains "evif_cache" then
        set output to output & GREEN & " Cache metrics: available" & lineBreak()
    else
        set output to output & YELLOW & " Cache metrics: not yet implemented" & lineBreak()
    end if

    -- Check for request latency metrics
    if metrics contains "evif_mcp_call_duration" then
        set output to output & GREEN & " MCP call duration: available" & lineBreak()
    else
        set output to output & YELLOW & " MCP call duration: not yet implemented" & lineBreak()
    end if

    -- Check for connection metrics
    if metrics contains "evif_active_connections" then
        set output to output & GREEN & " Connection metrics: available" & lineBreak()
    else
        set output to output & YELLOW & " Connection metrics: not yet implemented" & lineBreak()
    end if

    -- Check for tools called counter
    if metrics contains "evif_mcp_tools_called" then
        set output to output & GREEN & " Tools called counter: available" & lineBreak()
    else
        set output to output & YELLOW & " Tools called counter: not yet implemented" & lineBreak()
    end if

    return output
end checkPerformanceMetrics

-- Step 3: Prometheus Metrics
on checkPrometheusMetrics()
    set output to ""

    set metricsCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/metrics' 2>/dev/null"
    set metrics to do shell script metricsCmd

    -- Count Prometheus format lines
    set lineCount to do shell script "echo '" & metrics & "' | grep -c '^evif_' 2>/dev/null || echo 0"

    if lineCount > 10 then
        set output to output & GREEN & " Prometheus metrics: " & lineCount & " metrics available" & lineBreak()
    else if lineCount > 0 then
        set output to output & YELLOW & " Prometheus metrics: " & lineCount & " metrics (basic)" & lineBreak()
    else
        set output to output & YELLOW & " Prometheus metrics: not yet implemented" & lineBreak()
    end if

    -- List available metric types
    set output to output & lineBreak() & "  Available metrics:" & lineBreak()
    set metricTypes to do shell script "echo '" & metrics & "' | grep '^evif_' | cut -d' ' -f1 | sort -u | head -10"
    set AppleScript's text item delimiters to return
    set metricLines to text items of metricTypes

    repeat with metricName in metricLines
        if (length of (metricName as string)) > 0 then
            set output to output & "    - " & metricName & lineBreak()
        end if
    end repeat

    return output
end checkPrometheusMetrics

-- Step 4: MCP HTTP Features
on checkMCPHttpFeatures()
    set output to ""

    -- MCP Tools List
    set toolsCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/mcp/tools' 2>/dev/null"
    set tools to do shell script toolsCmd

    if tools contains "evif_ls" then
        set output to output & GREEN & " MCP tools list: available" & lineBreak()

        -- Count tools
        set toolCount to do shell script "echo '" & tools & "' | grep -o '\"name\"[[:space:]]*:[[:space:]]*\"[^\"]*\"' | wc -l | tr -d ' '"
        set output to output & "  Tools count: " & toolCount & lineBreak()
    else
        set output to output & YELLOW & " MCP tools list: not available" & lineBreak()
    end if

    -- Test MCP Tool Call
    set callCmd to "curl -s --max-time 10 --noproxy '*' -X POST 'http://localhost:8080/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_health\",\"args\":{}}' 2>/dev/null"
    set callResult to do shell script callCmd

    if callResult contains "success" or callResult contains "result" then
        set output to output & GREEN & " MCP tool call: working" & lineBreak()
    else
        set output to output & YELLOW & " MCP tool call: limited" & lineBreak()
    end if

    -- Test evif_ls call
    set lsCallCmd to "curl -s --max-time 10 --noproxy '*' -X POST 'http://localhost:8080/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_ls\",\"args\":{\"path\":\"/mem\"}}' 2>/dev/null"
    set lsResult to do shell script lsCallCmd

    if lsResult contains "success" then
        set output to output & GREEN & " evif_ls call: working" & lineBreak()
    else
        set output to output & YELLOW & " evif_ls call: check manually" & lineBreak()
    end if

    return output
end checkMCPHttpFeatures

-- Step 5: Token Optimization
on checkTokenOptimization()
    set output to ""

    set output to output & "Token optimization features:" & lineBreak()

    -- Test evif_cat with max_lines
    set catCmd to "curl -s --max-time 10 --noproxy '*' -X POST 'http://localhost:8080/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_cat\",\"args\":{\"path\":\"/mem/test.txt\",\"max_lines\":10,\"mode\":\"head\"}}' 2>/dev/null"
    set catResult to do shell script catCmd

    if catResult contains "success" or catResult contains "result" then
        set output to output & GREEN & " evif_cat max_lines: supported" & lineBreak()
    else
        set output to output & YELLOW & " evif_cat max_lines: check implementation" & lineBreak()
    end if

    -- Test memory search with compact
    set memCmd to "curl -s --max-time 10 --noproxy '*' -X POST 'http://localhost:8080/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_memory_search\",\"args\":{\"query\":\"test\",\"compact\":true,\"limit\":3}}' 2>/dev/null"
    set memResult to do shell script memCmd

    if memResult contains "success" or memResult contains "result" then
        set output to output & GREEN & " memory_search compact: supported" & lineBreak()
    else
        set output to output & YELLOW & " memory_search compact: check implementation" & lineBreak()
    end if

    set output to output & lineBreak() & "  Expected savings: 60-90%" & lineBreak()

    return output
end checkTokenOptimization

-- Step 6: SSE Readiness
on checkSSEReadiness()
    set output to ""

    -- Check SSE endpoint availability
    set sseCmd to "curl -s --max-time 5 --noproxy '*' -o /dev/null -w '%{http_code}' 'http://localhost:8080/api/v1/mcp/sse' 2>/dev/null"
    set sseCode to do shell script sseCmd

    if sseCode is "200" then
        set output to output & GREEN & " SSE endpoint: available" & lineBreak()
    else if sseCode is "404" then
        set output to output & YELLOW & " SSE endpoint: not yet implemented" & lineBreak()
    else
        set output to output & YELLOW & " SSE endpoint: status " & sseCode & lineBreak()
    end if

    -- Check real-time file watch
    set watchCmd to "curl -s --max-time 5 --noproxy '*' -o /dev/null -w '%{http_code}' 'http://localhost:8080/api/v1/watch?path=/mem' 2>/dev/null"
    set watchCode to do shell script watchCmd

    if watchCode is "200" then
        set output to output & GREEN & " File watch endpoint: available" & lineBreak()
    else
        set output to output & YELLOW & " File watch endpoint: not yet implemented" & lineBreak()
    end if

    return output
end checkSSEReadiness

-- Step 7: Authentication
on checkAuthentication()
    set output to ""

    -- Test unprotected endpoint (should work)
    set publicCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/health' 2>/dev/null"
    set publicResult to do shell script publicCmd

    if publicResult contains "healthy" then
        set output to output & GREEN & " Public endpoints: working" & lineBreak()
    else
        set output to output & RED & " Public endpoints: FAIL" & lineBreak()
    end if

    -- Test MCP tools without auth (should work currently)
    set mcpCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/mcp/tools' 2>/dev/null"
    set mcpResult to do shell script mcpCmd

    if mcpResult contains "evif_ls" then
        set output to output & YELLOW & " MCP tools: open access (auth not yet required)" & lineBreak()
    else
        set output to output & YELLOW & " MCP tools: check manually" & lineBreak()
    end if

    -- Check token generation endpoint
    set tokenCmd to "curl -s --max-time 5 --noproxy '*' -X POST 'http://localhost:8080/api/v1/auth/token' 2>/dev/null"
    set tokenResult to do shell script tokenCmd

    if tokenResult contains "token" or tokenResult contains "error" then
        set output to output & GREEN & " Token endpoint: available" & lineBreak()
    else
        set output to output & YELLOW & " Token endpoint: not yet implemented" & lineBreak()
    end if

    return output
end checkAuthentication

-- Step 8: Health Deep Check
on checkHealthDeep()
    set output to ""

    set healthCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/health' 2>/dev/null"
    set health to do shell script healthCmd

    if health contains "healthy" then
        set output to output & GREEN & " Health check: OK" & lineBreak()

        -- Parse uptime
        if health contains "uptime" then
            set uptime to do shell script "echo '" & health & "' | grep -o '\"uptime\"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*'"
            set output to output & "  Server uptime: " & uptime & " seconds" & lineBreak()
        end if

        -- Parse version
        if health contains "version" then
            set version to do shell script "echo '" & health & "' | grep -o '\"version\"[[:space:]]*:[[:space:]]*\"[^\"]*\"' | grep -o '\"[^\"]*\"$' | tr -d '\"'"
            set output to output & "  Version: " & version & lineBreak()
        end if
    else
        set output to output & RED & " Health check: FAIL" & lineBreak()
    end if

    -- Context layers check
    set ctxCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/directories?path=/context' 2>/dev/null"
    set ctx to do shell script ctxCmd

    if ctx contains "L0" and ctx contains "L1" and ctx contains "L2" then
        set output to output & GREEN & " Context layers (L0/L1/L2): OK" & lineBreak()
    else
        set output to output & YELLOW & " Context layers: partial" & lineBreak()
    end if

    -- Skills check
    set evifRoot to "/Users/louloulin/Documents/linchong/claude/evif"
    set skillsDir to evifRoot & "/.claude/skills"

    try
        set skillCount to do shell script "ls -la " & skillsDir & "/*.SKILL.md 2>/dev/null | wc -l | tr -d ' '"

        if skillCount > 0 then
            set output to output & GREEN & " Skills: " & skillCount & " skills available" & lineBreak()
        else
            set output to output & YELLOW & " Skills: none found" & lineBreak()
        end if
    on error
        set output to output & YELLOW & " Skills: directory not found" & lineBreak()
    end try

    return output
end checkHealthDeep

-- Helper: Line break
on lineBreak()
    return "
"
end lineBreak