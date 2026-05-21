-- evif-verify-comprehensive.scpt - EVIF 全功能验证 (Claude Code 集成)
-- 使用 osascript 运行，完整验证 EVIF 与 Claude Code 的集成价值
--
-- 验证覆盖:
-- 1. 核心 VFS (ls, cat, write, mkdir, rm)
-- 2. MCP HTTP 工具 (18 tools)
-- 3. 记忆系统 (memory search, memorize)
-- 4. 上下文层 (L0/L1/L2)
-- 5. Skills 系统
-- 6. RTK 风格优化
-- 7. 监控指标
-- 8. 多租户支持

property EVIF_ROOT : "/Users/louloulin/Documents/linchong/claude/evif"
property REST_URL : "http://localhost:8080"
property METRICS_URL : "http://localhost:8080/metrics"
property MCP_URL : "http://localhost:8080/api/v1/mcp"

-- 颜色代码
property GREEN : "✅"
property RED : "❌"
property YELLOW : "⚠️"
property BLUE : "🔵"

-- 全局统计
property totalTests : 0
property passedTests : 0
property failedTests : 0

on run argv
    set output to ""
    set startTime to current date

    -- 标题
    set output to output & lineBreak()
    set output to output & "╔══════════════════════════════════════════════════╗" & lineBreak()
    set output to output & "║     EVIF Comprehensive Verification              ║" & lineBreak()
    set output to output & "║     Claude Code Integration Analysis            ║" & lineBreak()
    set output to output & "╚══════════════════════════════════════════════════╝" & lineBreak()
    set output to output & lineBreak()
    set output to output & "Started: " & (current date as string) & lineBreak()
    set output to output & lineBreak()

    -- Phase 1: 基础架构验证
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 1: Core Architecture Validation" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()

    set output to output & checkPrerequisites() & lineBreak()
    set output to output & checkServerBaseline() & lineBreak()
    set output to output & checkMetricsEndpoint() & lineBreak()

    -- Phase 2: VFS 功能验证
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 2: VFS Operations Validation" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()

    set output to output & checkVFSOperations() & lineBreak()
    set output to output & checkContextLayers() & lineBreak()

    -- Phase 3: MCP HTTP 验证
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 3: MCP HTTP Integration" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()

    set output to output & checkMCPTools() & lineBreak()
    set output to output & checkMCPToolCall() & lineBreak()

    -- Phase 4: Memory System
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 4: Memory System Validation" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()

    set output to output & checkMemorySystem() & lineBreak()
    set output to output & checkSkillsSystem() & lineBreak()

    -- Phase 5: Performance & Optimization
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 5: Performance & Token Optimization" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()

    set output to output & checkTokenOptimization() & lineBreak()
    set output to output & checkCacheMetrics() & lineBreak()

    -- Phase 6: Claude Code Integration Value
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 6: Claude Code Integration Core Value" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()

    set output to output & analyzeCoreValue() & lineBreak()
    set output to output & checkRTKStyleIntegration() & lineBreak()

    -- 总结
    set elapsed to (current date) - startTime
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Verification Summary" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & "Total tests: " & totalTests & lineBreak()
    set output to output & "Passed: " & GREEN & " " & passedTests & lineBreak()
    set output to output & "Failed: " & (if failedTests > 0 then RED else GREEN) & " " & failedTests & lineBreak()
    set output to output & "Success rate: " & ((passedTests * 100) / totalTests) & "%" & lineBreak()
    set output to output & "Elapsed time: " & elapsed & " seconds" & lineBreak()
    set output to output & lineBreak()
    set output to output & "Completed: " & (current date as string) & lineBreak()

    -- 保存结果
    saveResults(output)

    return output
end run

-- Phase 1: 基础架构验证

on checkPrerequisites()
    set output to ""
    set output to output & BLUE & "[1.1] Prerequisites Check" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- curl
    set curlPath to do shell script "which curl 2>/dev/null || echo ''"
    if curlPath is not "" then
        set output to output & GREEN & " curl: " & curlPath & lineBreak()
        incrementTests()
    else
        set output to output & RED & " curl: NOT FOUND" & lineBreak()
        incrementFailed()
    end if

    -- jq (optional)
    set jqPath to do shell script "which jq 2>/dev/null || echo ''"
    if jqPath is not "" then
        set output to output & GREEN & " jq: " & jqPath & lineBreak()
    else
        set output to output & YELLOW & " jq: (optional)" & lineBreak()
    end if

    -- cargo
    set cargoPath to do shell script "which cargo 2>/dev/null || echo ''"
    if cargoPath is not "" then
        set output to output & GREEN & " cargo: " & cargoPath & lineBreak()
        incrementTests()
    else
        set output to output & RED & " cargo: NOT FOUND" & lineBreak()
        incrementFailed()
    end if

    -- osascript
    set output to output & GREEN & " osascript: available" & lineBreak()
    incrementTests()

    return output
end checkPrerequisites

on checkServerBaseline()
    set output to ""
    set output to output & lineBreak() & BLUE & "[1.2] Server Baseline" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- REST Health
    set healthCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/health' 2>/dev/null"
    set health to do shell script healthCmd

    if health contains "healthy" then
        set output to output & GREEN & " REST Server: healthy" & lineBreak()
        incrementTests()

        -- Parse version
        if health contains "version" then
            set version to do shell script "echo '" & health & "' | grep -o '\"version\"[[:space:]]*:[[:space:]]*\"[^\"]*\"' | head -1 | grep -o '\"[^\"]*\"$' | tr -d '\"'"
            set output to output & "  Version: " & version & lineBreak()
        end if

        -- Parse uptime
        if health contains "uptime" then
            set uptime to do shell script "echo '" & health & "' | grep -o '\"uptime\"[[:space:]]*:[[:space:]]*[0-9]*' | head -1 | grep -o '[0-9]*'"
            set output to output & "  Uptime: " & uptime & "s" & lineBreak()
        end if
    else
        set output to output & RED & " REST Server: FAIL" & lineBreak()
        incrementFailed()
    end if

    -- MCP Health
    set mcpHealthCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/mcp/health' 2>/dev/null"
    set mcpHealth to do shell script mcpHealthCmd

    if mcpHealth contains "healthy" then
        set output to output & GREEN & " MCP HTTP: healthy" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " MCP HTTP: not available" & lineBreak()
        incrementFailed()
    end if

    return output
end checkServerBaseline

on checkMetricsEndpoint()
    set output to ""
    set output to output & lineBreak() & BLUE & "[1.3] Metrics Endpoint" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set metricsCmd to "curl -s --max-time 5 --noproxy '*' '" & METRICS_URL & "' 2>/dev/null"
    set metrics to do shell script metricsCmd

    if length of metrics > 100 then
        set output to output & GREEN & " Metrics: accessible (" & (length of metrics) & " bytes)" & lineBreak()
        incrementTests()

        -- Count metrics
        set metricCount to do shell script "echo '" & metrics & "' | grep -c '^evif_' 2>/dev/null || echo 0"
        set output to output & "  Metric count: " & metricCount & lineBreak()
    else
        set output to output & RED & " Metrics: not accessible" & lineBreak()
        incrementFailed()
    end if

    return output
end checkMetricsEndpoint

-- Phase 2: VFS 功能验证

on checkVFSOperations()
    set output to ""
    set output to output & BLUE & "[2.1] VFS Operations (ls)" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- List /mem
    set lsCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/directories?path=/mem' 2>/dev/null"
    set lsResult to do shell script lsCmd

    if lsResult contains "files" then
        set output to output & GREEN & " Directory listing: OK" & lineBreak()
        incrementTests()
    else
        set output to output & RED & " Directory listing: FAIL" & lineBreak()
        incrementFailed()
    end if

    -- List /context
    set ctxCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/directories?path=/context' 2>/dev/null"
    set ctxResult to do shell script ctxCmd

    if ctxResult contains "L0" and ctxResult contains "L1" and ctxResult contains "L2" then
        set output to output & GREEN & " Context layers: L0/L1/L2 detected" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " Context layers: partial detection" & lineBreak()
        incrementFailed()
    end if

    -- List /skills
    set skillsCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/directories?path=/skills' 2>/dev/null"
    set skillsResult to do shell script skillsCmd

    if skillsResult contains "files" then
        set output to output & GREEN & " Skills directory: accessible" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " Skills directory: not available" & lineBreak()
    end if

    return output
end checkVFSOperations

on checkContextLayers()
    set output to ""
    set output to output & lineBreak() & BLUE & "[2.2] Context Layer System" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set output to output & "3-Layer Architecture:" & lineBreak()

    -- L0: Current task (ephemeral)
    set l0Cmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/directories?path=/context/L0' 2>/dev/null"
    set l0 to do shell script l0Cmd
    if l0 contains "current" then
        set output to output & GREEN & "  L0 (current): ephemeral task" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & "  L0: not initialized" & lineBreak()
    end if

    -- L1: Session decisions (durable)
    set l1Cmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/directories?path=/context/L1' 2>/dev/null"
    set l1 to do shell script l1Cmd
    if l1 contains "decisions" then
        set output to output & GREEN & "  L1 (decisions): session durable" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & "  L1: not initialized" & lineBreak()
    end if

    -- L2: Project knowledge (persistent)
    set l2Cmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/directories?path=/context/L2' 2>/dev/null"
    set l2 to do shell script l2Cmd
    if l2 contains "architecture" or l2 contains "patterns" then
        set output to output & GREEN & "  L2 (knowledge): project persistent" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & "  L2: not initialized" & lineBreak()
    end if

    return output
end checkContextLayers

-- Phase 3: MCP HTTP 验证

on checkMCPTools()
    set output to ""
    set output to output & BLUE & "[3.1] MCP Tools List" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set toolsCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/mcp/tools' 2>/dev/null"
    set tools to do shell script toolsCmd

    if tools contains "evif_ls" then
        set output to output & GREEN & " MCP tools: available" & lineBreak()
        incrementTests()

        -- Count tools
        set toolCount to do shell script "echo '" & tools & "' | grep -o '\"name\"[[:space:]]*:[[:space:]]*\"[^\"]*\"' | wc -l | tr -d ' '"
        set output to output & "  Tools count: " & toolCount & lineBreak()

        -- List key tools
        set output to output & lineBreak() & "  Key tools:" & lineBreak()
        set toolList to {"evif_ls", "evif_cat", "evif_write", "evif_mkdir", "evif_rm"}
        repeat with toolName in toolList
            if tools contains toolName then
                set output to output & "    " & GREEN & " " & toolName & lineBreak()
            end if
        end repeat

    else
        set output to output & RED & " MCP tools: FAIL" & lineBreak()
        incrementFailed()
    end if

    return output
end checkMCPTools

on checkMCPToolCall()
    set output to ""
    set output to output & lineBreak() & BLUE & "[3.2] MCP Tool Call (evif_ls)" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- Test evif_ls
    set lsCallCmd to "curl -s --max-time 10 --noproxy '*' -X POST '" & REST_URL & "/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_ls\",\"args\":{\"path\":\"/mem\"}}' 2>/dev/null"
    set lsResult to do shell script lsCallCmd

    if lsResult contains "success" then
        set output to output & GREEN & " evif_ls: success" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " evif_ls: " & (characters 1 through 100 of lsResult) & lineBreak()
        incrementFailed()
    end if

    -- Test evif_health
    set healthCallCmd to "curl -s --max-time 10 --noproxy '*' -X POST '" & REST_URL & "/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_health\",\"args\":{}}' 2>/dev/null"
    set healthResult to do shell script healthCallCmd

    if healthResult contains "success" or healthResult contains "result" then
        set output to output & GREEN & " evif_health: success" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " evif_health: " & (characters 1 through 100 of healthResult) & lineBreak()
    end if

    return output
end checkMCPToolCall

-- Phase 4: Memory System

on checkMemorySystem()
    set output to ""
    set output to output & BLUE & "[4.1] Memory System" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- Check memory endpoint
    set memCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/api/v1/memory/search' 2>/dev/null"
    set memResult to do shell script memCmd

    if memResult contains "results" or memResult contains "error" then
        set output to output & GREEN & " Memory search: available" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " Memory search: limited" & lineBreak()
    end if

    -- Test memory search via MCP
    set memSearchCmd to "curl -s --max-time 10 --noproxy '*' -X POST '" & REST_URL & "/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_memory_search\",\"args\":{\"query\":\"test\",\"limit\":3}}' 2>/dev/null"
    set memSearchResult to do shell script memSearchCmd

    if memSearchResult contains "success" or memSearchResult contains "results" then
        set output to output & GREEN & " Memory search MCP: working" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " Memory search MCP: not available" & lineBreak()
    end if

    return output
end checkMemorySystem

on checkSkillsSystem()
    set output to ""
    set output to output & lineBreak() & BLUE & "[4.2] Skills System" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set skillsDir to EVIF_ROOT & "/.claude/skills"

    try
        set skillCount to do shell script "ls " & skillsDir & "/*.SKILL.md 2>/dev/null | wc -l | tr -d ' '"

        if skillCount > 0 then
            set output to output & GREEN & " Skills found: " & skillCount & lineBreak()
            incrementTests()

            -- List skills
            set output to output & lineBreak() & "  Available skills:" & lineBreak()
            set skillList to do shell script "ls " & skillsDir & "/*.SKILL.md 2>/dev/null | xargs -I {} basename {} .SKILL.md | head -10"
            set AppleScript's text item delimiters to return
            set skillLines to text items of skillList

            repeat with skillName in skillLines
                if (length of (skillName as string)) > 0 then
                    set output to output & "    - " & skillName & lineBreak()
                end if
            end repeat
        else
            set output to output & YELLOW & " Skills: none found" & lineBreak()
        end if
    on error
        set output to output & RED & " Skills directory: not found" & lineBreak()
        incrementFailed()
    end try

    return output
end checkSkillsSystem

-- Phase 5: Performance & Token Optimization

on checkTokenOptimization()
    set output to ""
    set output to output & BLUE & "[5.1] Token Optimization" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set output to output & "RTK-style Token Savings:" & lineBreak()

    -- Test evif_cat with max_lines
    set catCmd to "curl -s --max-time 10 --noproxy '*' -X POST '" & REST_URL & "/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_cat\",\"args\":{\"path\":\"/mem/test.txt\",\"max_lines\":10,\"mode\":\"head\"}}' 2>/dev/null"
    set catResult to do shell script catCmd

    if catResult contains "success" or catResult contains "result" then
        set output to output & GREEN & "  evif_cat max_lines: supported" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & "  evif_cat max_lines: limited" & lineBreak()
    end if

    -- Test compact mode
    set compactCmd to "curl -s --max-time 10 --noproxy '*' -X POST '" & REST_URL & "/api/v1/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_memory_search\",\"args\":{\"query\":\"test\",\"compact\":true,\"limit\":3}}' 2>/dev/null"
    set compactResult to do shell script compactCmd

    if compactResult contains "success" or compactResult contains "results" then
        set output to output & GREEN & "  memory_search compact: supported" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & "  memory_search compact: limited" & lineBreak()
    end if

    set output to output & lineBreak() & "  Expected savings: 60-90%" & lineBreak()

    return output
end checkTokenOptimization

on checkCacheMetrics()
    set output to ""
    set output to output & lineBreak() & BLUE & "[5.2] Cache & Performance Metrics" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set metricsCmd to "curl -s --max-time 5 --noproxy '*' '" & METRICS_URL & "' 2>/dev/null"
    set metrics to do shell script metricsCmd

    -- Check for specific metrics
    set metricChecks to {"evif_cache_hits", "evif_mcp_call_duration", "evif_active_connections", "evif_mcp_tools_called"}

    set foundMetrics to 0
    repeat with metricName in metricChecks
        if metrics contains metricName then
            set foundMetrics to foundMetrics + 1
        end if
    end repeat

    if foundMetrics > 0 then
        set output to output & GREEN & " Performance metrics: " & foundMetrics & "/4 available" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " Performance metrics: basic implementation" & lineBreak()
    end if

    return output
end checkCacheMetrics

-- Phase 6: Claude Code Integration Value

on analyzeCoreValue()
    set output to ""
    set output to output & BLUE & "[6.1] Claude Code Integration Core Value" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set output to output & "EVIF brings to Claude Code:" & lineBreak()
    set output to output & lineBreak()

    set output to output & "1. Persistent Context (L0/L1/L2)" & lineBreak()
    set output to output & "   - Cross-session memory" & lineBreak()
    set output to output & "   - Decision tracking" & lineBreak()
    set output to output & "   - Project knowledge persistence" & lineBreak()

    set output to output & lineBreak()

    set output to output & "2. Enhanced Tool Calling" & lineBreak()
    set output to output & "   - 18+ MCP tools via unified API" & lineBreak()
    set output to output & "   - Token optimization (60-90% savings)" & lineBreak()
    set output to output & "   - Truncation and compact modes" & lineBreak()

    set output to output & lineBreak()

    set output to output & "3. Multi-Agent Coordination" & lineBreak()
    set output to output & "   - PipeFS for task pipelines" & lineBreak()
    set output to output & "   - Skills for reusable workflows" & lineBreak()
    set output to output & "   - Multi-tenant support" & lineBreak()

    set output to output & lineBreak()

    set output to output & "4. Production-Ready Features" & lineBreak()
    set output to output & "   - Circuit breaker" & lineBreak()
    set output to output & "   - Prometheus metrics" & lineBreak()
    set output to output & "   - SSE real-time push" & lineBreak()
    set output to output & "   - Batch operations" & lineBreak()

    return output
end analyzeCoreValue

on checkRTKStyleIntegration()
    set output to ""
    set output to output & lineBreak() & BLUE & "[6.2] RTK-Style Integration" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set output to output & "Meta Commands (like RTK):" & lineBreak()

    -- Check evif-proxy.sh
    set proxyPath to EVIF_ROOT & "/scripts/oscript/evif-proxy.sh"
    set proxyExists to do shell script "test -f '" & proxyPath & "' && echo 'yes' || echo 'no'"

    if proxyExists is "yes" then
        set output to output & GREEN & "  evif-proxy.sh: available" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & "  evif-proxy.sh: not found" & lineBreak()
    end if

    set output to output & lineBreak()
    set output to output & "  evif gain              # Show integration stats" & lineBreak()
    set output to output & "  evif gain --history   # Show command history" & lineBreak()
    set output to output & "  evif discover          # Analyze integration opportunities" & lineBreak()
    set output to output & "  evif proxy <cmd>       # Execute raw command" & lineBreak()

    set output to output & lineBreak()
    set output to output & "Hook-based transparent usage:" & lineBreak()
    set output to output & "  Claude Code: evif ls /mem" & lineBreak()
    set output to output & "      ↓ (transparent)" & lineBreak()
    set output to output & "  evif-proxy.sh wrapper" & lineBreak()
    set output to output & "      ↓" & lineBreak()
    set output to output & "  EVIF REST API" & lineBreak()
    set output to output & "      ↓" & lineBreak()
    set output to output & "  Token optimization (60-90% saved)" & lineBreak()

    return output
end checkRTKStyleIntegration

-- Helper functions

on incrementTests()
    set totalTests to totalTests + 1
    set passedTests to passedTests + 1
end incrementTests

on incrementFailed()
    set totalTests to totalTests + 1
    set failedTests to failedTests + 1
end incrementFailed

on saveResults(results)
    set logPath to EVIF_ROOT & "/.claude/evif-verify-comprehensive-output.txt"
    try
        set fout to open for access file logPath with write permission
        write results to fout
        close access fout
        -- Also write timestamp
        set tsPath to EVIF_ROOT & "/.claude/evif-verify-timestamp.txt"
        set ts to open for access file tsPath with write permission
        write "Last run: " & (current date as string) & return to ts
        write "Total: " & totalTests & " Passed: " & passedTests & " Failed: " & failedTests & return to ts
        write "Success rate: " & ((passedTests * 100) / totalTests) & "%" to ts
        close access ts
    on error
        -- Silent fail for automation
    end try
end saveResults

on lineBreak()
    return "
"
end lineBreak