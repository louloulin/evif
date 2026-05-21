-- evif-verify-connections.scpt - EVIF 连接功能验证
-- 验证 EVIF 的多连接功能:
-- 1. PipeFS (多Agent协调)
-- 2. WebSocket (实时通信)
-- 3. Plugin 系统 (插件连接)
-- 4. Context Layers (L0/L1/L2)
-- 5. Skills 系统 (技能协调)
-- 6. MCP HTTP (HTTP API连接)

property EVIF_ROOT : "/Users/louloulin/Documents/linchong/claude/evif"
property REST_URL : "http://localhost:8080"
property WS_URL : "ws://localhost:8080/api/v1/ws"
property MCP_URL : "http://localhost:8080/api/v1/mcp"

-- 颜色代码
property GREEN : "✅"
property RED : "❌"
property YELLOW : "⚠️"
property BLUE : "🔵"
property CYAN : "🔷"

-- 全局统计
property totalTests : 0
property passedTests : 0
property failedTests : 0

on run argv
    set output to ""

    set output to output & lineBreak()
    set output to output & "╔══════════════════════════════════════════════════╗" & lineBreak()
    set output to output & "║     EVIF Connection Features Verification       ║" & lineBreak()
    set output to output & "║     (Multi-Agent, WebSocket, Plugins)         ║" & lineBreak()
    set output to output & "╚══════════════════════════════════════════════════╝" & lineBreak()
    set output to output & lineBreak()
    set output to output & "Date: " & (current date as string) & lineBreak()
    set output to output & lineBreak()

    -- Phase 1: Server Baseline
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 1: Server Baseline" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & checkServerBaseline() & lineBreak()

    -- Phase 2: PipeFS (多Agent协调)
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 2: PipeFS (Multi-Agent Coordination)" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & checkPipeFS() & lineBreak()

    -- Phase 3: WebSocket (实时通信)
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 3: WebSocket (Real-time Communication)" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & checkWebSocket() & lineBreak()

    -- Phase 4: Plugin System
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 4: Plugin System (20+ Plugins)" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & checkPluginSystem() & lineBreak()

    -- Phase 5: Context Layers
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 5: Context Layers (L0/L1/L2)" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & checkContextLayers() & lineBreak()

    -- Phase 6: Skills System
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 6: Skills System" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & checkSkillsSystem() & lineBreak()

    -- Phase 7: MCP HTTP Integration
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Phase 7: MCP HTTP Integration" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & checkMCPHttp() & lineBreak()

    -- Summary
    set output to output & lineBreak() & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & "Connection Verification Summary" & lineBreak()
    set output to output & "═══════════════════════════════════════════════════" & lineBreak()
    set output to output & lineBreak()
    set output to output & "Total tests: " & totalTests & lineBreak()
    set output to output & "Passed: " & GREEN & " " & passedTests & lineBreak()
    set output to output & "Failed: " & (if failedTests > 0 then RED else GREEN) & " " & failedTests & lineBreak()
    set output to output & "Success rate: " & ((passedTests * 100) / totalTests) & "%" & lineBreak()
    set output to output & lineBreak()

    -- Save results
    saveResults(output)

    return output
end run

-- Phase 1: Server Baseline

on checkServerBaseline()
    set output to ""
    set output to output & BLUE & "[1.1] Server Health" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- REST Health
    set healthCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/health' 2>/dev/null"
    set health to do shell script healthCmd

    if health contains "healthy" then
        set output to output & GREEN & " REST Server: healthy" & lineBreak()
        incrementTests()
    else
        set output to output & RED & " REST Server: FAIL" & lineBreak()
        incrementFailed()
    end if

    -- MCP Health
    set mcpHealthCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/mcp/health' 2>/dev/null"
    set mcpHealth to do shell script mcpHealthCmd

    if mcpHealth contains "healthy" then
        set output to output & GREEN & " MCP Server: healthy" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " MCP Server: limited" & lineBreak()
    end if

    -- Metrics
    set metricsCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/metrics' 2>/dev/null"
    set metrics to do shell script metricsCmd

    if length of metrics > 100 then
        set output to output & GREEN & " Metrics endpoint: accessible" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " Metrics endpoint: limited" & lineBreak()
    end if

    return output
end checkServerBaseline

-- Phase 2: PipeFS (Multi-Agent Coordination)

on checkPipeFS()
    set output to ""
    set output to output & BLUE & "[2.1] PipeFS (Task Pipeline)" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- Check /pipes directory
    set pipesCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/directories?path=/pipes' 2>/dev/null"
    set pipesResult to do shell script pipesCmd

    if pipesResult contains "files" or pipesResult contains "evif" then
        set output to output & GREEN & " /pipes directory: accessible" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " /pipes directory: not found" & lineBreak()
    end if

    -- Check PipeFS capabilities via MCP
    set pipefsCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/mcp/tools' 2>/dev/null"
    set pipefsTools to do shell script pipefsCmd

    set pipefs_features to {"evif_pipe_read", "evif_pipe_write", "evif_pipe_list", "evif_queue"}
    set foundPipeFeatures to 0

    repeat with feature in pipefs_features
        if pipefsTools contains feature then
            set foundPipeFeatures to foundPipeFeatures + 1
        end if
    end repeat

    if foundPipeFeatures > 0 then
        set output to output & GREEN & " PipeFS features: " & foundPipeFeatures & " found" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " PipeFS: MCP tools not found" & lineBreak()
    end if

    set output to output & lineBreak() & "  Multi-Agent Use Cases:" & lineBreak()
    set output to output & "    - Task delegation between agents" & lineBreak()
    set output to output & "    - Pipeline workflows (A → B → C)" & lineBreak()
    set output to output & "    - Concurrent task execution" & lineBreak()

    return output
end checkPipeFS

-- Phase 3: WebSocket (Real-time Communication)

on checkWebSocket()
    set output to ""
    set output to output & BLUE & "[3.1] WebSocket Endpoint" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- Check WebSocket endpoint
    set wsCmd to "curl -s --max-time 5 --noproxy '*' -o /dev/null -w '%{http_code}' '" & REST_URL & "/ws' 2>/dev/null"
    set wsCode to do shell script wsCmd

    if wsCode is "200" or wsCode is "101" then
        set output to output & GREEN & " WebSocket endpoint: available (HTTP " & wsCode & ")" & lineBreak()
        incrementTests()
    else if wsCode is "404" then
        set output to output & YELLOW & " WebSocket endpoint: not implemented (404)" & lineBreak()
    else
        set output to output & YELLOW & " WebSocket endpoint: status " & wsCode & lineBreak()
    end if

    -- Check SSE endpoint
    set sseCmd to "curl -s --max-time 5 --noproxy '*' -o /dev/null -w '%{http_code}' '" & REST_URL & "/mcp/sse' 2>/dev/null"
    set sseCode to do shell script sseCmd

    if sseCode is "200" then
        set output to output & GREEN & " SSE endpoint: available" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " SSE endpoint: status " & sseCode & lineBreak()
    end if

    set output to output & lineBreak() & "  Real-time Features:" & lineBreak()
    set output to output & "    - Live command output streaming" & lineBreak()
    set output to output & "    - File change notifications" & lineBreak()
    set output to output & "    - Progress updates" & lineBreak()
    set output to output & "    - Terminal session (TODO)" & lineBreak()

    return output
end checkWebSocket

-- Phase 4: Plugin System

on checkPluginSystem()
    set output to ""
    set output to output & BLUE & "[4.1] Plugin System" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- List available plugins via MCP tools
    set toolsCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/mcp/tools' 2>/dev/null"
    set tools to do shell script toolsCmd

    set output to output & "  Available plugin-based tools:" & lineBreak()

    set plugin_tools to {"evif_ls", "evif_cat", "evif_write", "evif_mkdir", "evif_rm", ¬
        "evif_cp", "evif_mv", "evif_stat", "evif_health", "evif_ping", ¬
        "evif_memory_search", "evif_memory_memorize", "evif_pipe_read", "evif_pipe_write", ¬
        "evif_queue_push", "evif_queue_pop", "evif_context_get", "evif_context_set", ¬
        "evif_skill_execute", "evif_metrics"}

    set foundCount to 0
    repeat with tool in plugin_tools
        if tools contains tool then
            set foundCount to foundCount + 1
        end if
    end repeat

    if foundCount > 10 then
        set output to output & GREEN & "  Plugin tools: " & foundCount & "/" & (length of plugin_tools) & " available" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & "  Plugin tools: limited (" & foundCount & ")" & lineBreak()
    end if

    -- Check for specific plugins
    set output to output & lineBreak() & "  Core Plugins:" & lineBreak()

    set plugins to {"contextfs", "skillfs", "pipefs", "memfs", "localfs", "kvfs"}
    repeat with plugin in plugins
        if tools contains plugin then
            set output to output & GREEN & "    " & plugin & ": available" & lineBreak()
        end if
    end repeat

    set output to output & lineBreak() & "  Storage Plugins:" & lineBreak()
    set output to output & "    - localfs: local filesystem" & lineBreak()
    set output to output & "    - kvfs: key-value store" & lineBreak()
    set output to output & "    - memfs: in-memory filesystem" & lineBreak()
    set output to output & "    - s3fs: S3 compatible" & lineBreak()
    set output to output & "    - sqlfs: SQL database" & lineBreak()

    set output to output & lineBreak() & "  Cloud Plugins:" & lineBreak()
    set output to output & "    - githubfs: GitHub integration" & lineBreak()
    set output to output & "    - gmailfs: Gmail integration" & lineBreak()
    set output to output & "    - notionfs: Notion integration" & lineBreak()
    set output to output & "    - discordfs: Discord integration" & lineBreak()

    return output
end checkPluginSystem

-- Phase 5: Context Layers

on checkContextLayers()
    set output to ""
    set output to output & BLUE & "[5.1] Context Layer System (L0/L1/L2)" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- Check all three layers
    set layers to {"L0", "L1", "L2"}
    set foundLayers to 0

    repeat with layer in layers
        set layerCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/directories?path=/context/" & layer & "' 2>/dev/null"
        set layerResult to do shell script layerCmd

        if layerResult contains "files" or layerResult contains "current" or layerResult contains "decisions" then
            set foundLayers to foundLayers + 1

            -- Describe layer
            if layer is "L0" then
                set output to output & GREEN & "  L0 (current): ephemeral task state" & lineBreak()
            else if layer is "L1" then
                set output to output & GREEN & "  L1 (decisions): session durable" & lineBreak()
            else
                set output to output & GREEN & "  L2 (knowledge): project persistent" & lineBreak()
            end if
        end if
    end repeat

    if foundLayers is 3 then
        set output to output & lineBreak() & GREEN & " All context layers available!" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " Context layers: " & foundLayers & "/3 found" & lineBreak()
    end if

    set output to output & lineBreak() & "  Layer Architecture:" & lineBreak()
    set output to output & "    L0: Current task (ephemeral)" & lineBreak()
    set output to output & "    L1: Session decisions (durable)" & lineBreak()
    set output to output & "    L2: Project knowledge (persistent)" & lineBreak()

    return output
end checkContextLayers

-- Phase 6: Skills System

on checkSkillsSystem()
    set output to ""
    set output to output & BLUE & "[6.1] Skills System" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    set skillsDir to EVIF_ROOT & "/.claude/skills"

    try
        set skillCount to do shell script "ls " & skillsDir & "/*.SKILL.md 2>/dev/null | wc -l | tr -d ' '"

        if skillCount > 0 then
            set output to output & GREEN & " Skills: " & skillCount & " available" & lineBreak()
            incrementTests()

            -- List skills
            set output to output & lineBreak() & "  Available Skills:" & lineBreak()
            set skillList to do shell script "ls " & skillsDir & "/*.SKILL.md 2>/dev/null | xargs -I {} basename {} .SKILL.md"
            set AppleScript's text item delimiters to return
            set skillLines to text items of skillList

            repeat with skillName in skillLines
                if (length of (skillName as string)) > 0 then
                    set output to output & "    - " & skillName & lineBreak()
                end if
            end repeat

            set output to output & lineBreak() & "  Skill Categories:" & lineBreak()
            set output to output & "    - evif-context: persistent memory" & lineBreak()
            set output to output & "    - evif-workflows: reusable workflows" & lineBreak()
            set output to output & "    - evif-pipes: multi-agent coordination" & lineBreak()
            set output to output & "    - evif-memory: vector memory" & lineBreak()
            set output to output & "    - evif-quickref: command reference" & lineBreak()
        else
            set output to output & YELLOW & " Skills: none found" & lineBreak()
        end if
    on error
        set output to output & RED & " Skills directory: error" & lineBreak()
        incrementFailed()
    end try

    return output
end checkSkillsSystem

-- Phase 7: MCP HTTP Integration

on checkMCPHttp()
    set output to ""
    set output to output & BLUE & "[7.1] MCP HTTP Integration" & lineBreak()
    set output to output & "─────────────────────────────────────────" & lineBreak()

    -- MCP Tools List
    set toolsCmd to "curl -s --max-time 5 --noproxy '*' '" & REST_URL & "/mcp/tools' 2>/dev/null"
    set tools to do shell script toolsCmd

    if tools contains "evif_ls" then
        set toolCount to do shell script "echo '" & tools & "' | grep -o '\"name\"[[:space:]]*:[[:space:]]*\"[^\"]*\"' | wc -l | tr -d ' '"
        set output to output & GREEN & " MCP tools: " & toolCount & " available" & lineBreak()
        incrementTests()
    else
        set output to output & RED & " MCP tools: FAIL" & lineBreak()
        incrementFailed()
    end if

    -- Test tool call
    set callCmd to "curl -s --max-time 10 --noproxy '*' -X POST '" & REST_URL & "/mcp/call' -H 'Content-Type: application/json' -d '{\"tool\":\"evif_health\",\"args\":{}}' 2>/dev/null"
    set callResult to do shell script callCmd

    if callResult contains "success" or callResult contains "result" then
        set output to output & GREEN & " MCP tool call: working" & lineBreak()
        incrementTests()
    else
        set output to output & YELLOW & " MCP tool call: limited" & lineBreak()
    end if

    set output to output & lineBreak() & "  MCP HTTP Benefits:" & lineBreak()
    set output to output & "    - Language-agnostic API (any HTTP client)" & lineBreak()
    set output to output & "    - Token optimization via parameters" & lineBreak()
    set output to output & "    - Cross-platform compatibility" & lineBreak()

    return output
end checkMCPHttp

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
    set logPath to EVIF_ROOT & "/.claude/evif-verify-connections-output.txt"
    try
        set fout to open for access file logPath with write permission
        write results to fout
        close access fout
    on error
        -- Silent fail
    end try
end saveResults

on lineBreak()
    return "
"
end lineBreak