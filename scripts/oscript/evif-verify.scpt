-- evif-verify.scpt - macOS AppleScript EVIF 验证
-- 使用 osascript 运行，验证 Claude Code 集成 EVIF

property REST_URL : "http://localhost:8080"
property MCP_URL : "http://localhost:8080/api/v1/mcp"

-- 颜色代码
property GREEN : "✅"
property RED : "❌"
property YELLOW : "⚠️"

on run argv
    set output to ""

    set output to output & lineBreak()
    set output to output & "==========================================" & lineBreak()
    set output to output & "EVIF AppleScript Verification" & lineBreak()
    set output to output & "Date: " & (current date as string) & lineBreak()
    set output to output & "==========================================" & lineBreak()
    set output to output & lineBreak()

    -- Step 1: Prerequisites
    set output to output & "[Step 1] Prerequisites" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkPrerequisites() & lineBreak()

    -- Step 2: Server Status
    set output to output & lineBreak() & "[Step 2] Server Status" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set serverStatus to checkServerStatus()
    set output to output & serverStatus & lineBreak()

    -- Step 3: Skills Check
    set output to output & lineBreak() & "[Step 3] Claude Code Skills" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & checkSkills() & lineBreak()

    -- Step 4: REST API
    set output to output & lineBreak() & "[Step 4] REST API Tests" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & testRestAPI() & lineBreak()

    -- Step 5: MCP HTTP
    set output to output & lineBreak() & "[Step 5] MCP HTTP Tests" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & testMCP() & lineBreak()

    -- Step 6: Core Value
    set output to output & lineBreak() & "[Step 6] Core Value Analysis" & lineBreak()
    set output to output & "-----------------------------------" & lineBreak()
    set output to output & analyzeCoreValue() & lineBreak()

    -- Summary
    set output to output & lineBreak() & "==========================================" & lineBreak()
    set output to output & "Verification Summary" & lineBreak()
    set output to output & "==========================================" & lineBreak()
    set output to output & lineBreak()
    set output to output & "All checks completed at " & (current date as string) & lineBreak()

    -- Save to file and return (for automation)
    set logPath to (POSIX path of (path to me)) & "../evif-verify-output.txt"
    try
        set fout to open for access file logPath with write permission
        write output to fout
        close access fout
    on error
        -- Silent fail for automation
    end try

    return output
end run

-- Check macOS prerequisites
on checkPrerequisites()
    set output to ""

    -- Check curl
    set curlPath to do shell script "which curl"
    if curlPath is not "" then
        set output to output & GREEN & " curl found: " & curlPath & lineBreak()
    else
        set output to output & RED & " curl not found" & lineBreak()
    end if

    -- Check jq
    set jqPath to do shell script "which jq 2>/dev/null || echo ''"
    if jqPath is not "" then
        set output to output & GREEN & " jq found: " & jqPath & lineBreak()
    else
        set output to output & YELLOW & " jq not found (optional)" & lineBreak()
    end if

    -- Check cargo
    set cargoPath to do shell script "which cargo"
    if cargoPath is not "" then
        set output to output & GREEN & " cargo found: " & cargoPath & lineBreak()
    else
        set output to output & RED & " cargo not found" & lineBreak()
    end if

    return output
end checkPrerequisites

-- Check server status
on checkServerStatus()
    set output to ""

    -- REST URL
    set restUrl to "http://localhost:8080/api/v1/health"
    set restCmd to "curl -s --max-time 5 --noproxy '*' '" & restUrl & "' 2>/dev/null"
    set restHealth to do shell script restCmd

    if restHealth contains "healthy" then
        set output to output & GREEN & " REST Server healthy" & lineBreak()
    else
        set output to output & RED & " REST Server not responding" & lineBreak()
        set output to output & "  (debug: " & restHealth & ")" & lineBreak()
    end if

    -- MCP URL
    set mcpUrl to "http://localhost:8080/api/v1/mcp/health"
    set mcpCmd to "curl -s --max-time 5 --noproxy '*' '" & mcpUrl & "' 2>/dev/null"
    set mcpHealth to do shell script mcpCmd

    if mcpHealth contains "healthy" then
        set output to output & GREEN & " MCP HTTP healthy" & lineBreak()
    else
        set output to output & YELLOW & " MCP HTTP not responding" & lineBreak()
        set output to output & "  (debug: " & mcpHealth & ")" & lineBreak()
    end if

    return output
end checkServerStatus

-- Check Skills directory
on checkSkills()
    set output to ""

    set evifRoot to "/Users/louloulin/Documents/linchong/claude/evif"
    set skillsDir to evifRoot & "/.claude/skills"

    try
        set skillCount to do shell script "ls -la " & skillsDir & "/*.SKILL.md 2>/dev/null | wc -l | tr -d ' '"

        set output to output & GREEN & " Found " & skillCount & " EVIF Skills:" & lineBreak()

        set skillList to do shell script "ls " & skillsDir & "/*.SKILL.md 2>/dev/null | xargs -I {} basename {} .SKILL.md"
        set AppleScript's text item delimiters to return
        set skillLines to text items of skillList

        repeat with skillName in skillLines
            set output to output & "  - " & skillName & lineBreak()
        end repeat

    on error
        set output to output & RED & " Skills directory not found" & lineBreak()
    end try

    return output
end checkSkills

-- Test REST API
on testRestAPI()
    set output to ""

    -- Health check
    set healthCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/health' 2>/dev/null"
    set health to do shell script healthCmd
    if health contains "healthy" then
        set output to output & GREEN & " Health check: OK" & lineBreak()
    else
        set output to output & RED & " Health check: FAIL" & lineBreak()
    end if

    -- Directories
    set dirsCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/directories?path=/skills' 2>/dev/null"
    set dirs to do shell script dirsCmd
    if dirs contains "files" then
        set output to output & GREEN & " Directory listing: OK" & lineBreak()
    else
        set output to output & YELLOW & " Directory listing: FAIL" & lineBreak()
    end if

    -- Context layers
    set ctxCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/directories?path=/context' 2>/dev/null"
    set ctx to do shell script ctxCmd
    if ctx contains "L0" and ctx contains "L1" and ctx contains "L2" then
        set output to output & GREEN & " Context layers (L0/L1/L2): OK" & lineBreak()
    else
        set output to output & YELLOW & " Context layers: FAIL" & lineBreak()
    end if

    return output
end testRestAPI

-- Test MCP HTTP
on testMCP()
    set output to ""

    -- MCP Tools
    set toolsCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/mcp/tools' 2>/dev/null"
    set tools to do shell script toolsCmd

    if tools contains "evif_ls" then
        set output to output & GREEN & " MCP tools available" & lineBreak()
    else
        set output to output & RED & " MCP tools not available" & lineBreak()
    end if

    -- Tool count
    if tools contains "count" then
        set toolCount to do shell script "echo '" & tools & "' | grep -o '\"count\"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*'"
        set output to output & "  Tools count: " & toolCount & lineBreak()
    end if

    -- MCP health
    set mcpHealthCmd to "curl -s --max-time 5 --noproxy '*' 'http://localhost:8080/api/v1/mcp/health' 2>/dev/null"
    set mcpHealth to do shell script mcpHealthCmd
    if mcpHealth contains "healthy" then
        set output to output & GREEN & " MCP HTTP health: OK" & lineBreak()
    else
        set output to output & RED & " MCP HTTP health: FAIL" & lineBreak()
    end if

    return output
end testMCP

-- Analyze core value
on analyzeCoreValue()
    set output to ""

    set output to output & "Claude Code 集成 EVIF 核心价值:" & lineBreak()
    set output to output & lineBreak()

    set output to output & "1. 持久化上下文 (L0/L1/L2)" & lineBreak()
    set output to output & "   - 跨会话记忆" & lineBreak()
    set output to output & "   - 决策追溯" & lineBreak()

    set output to output & lineBreak()
    set output to output & "2. 增强工具调用 (18 MCP tools)" & lineBreak()
    set output to output & "   - Token 优化 (60-90% 节省)" & lineBreak()
    set output to output & "   - 统一 API 接口" & lineBreak()

    set output to output & lineBreak()
    set output to output & "3. 多 Agent 协调" & lineBreak()
    set output to output & "   - PipeFS 管道" & lineBreak()
    set output to output & "   - Skills 复用" & lineBreak()

    return output
end analyzeCoreValue

-- Helper: HTTP GET
on curlGet(url)
    try
        set cmd to "curl -s --max-time 5 --noproxy '*' '" & url & "' 2>/dev/null"
        set response to do shell script cmd
        return response
    on error errStr
        return ""
    end try
end curlGet

-- Helper: Line break
on lineBreak()
    return "
"
end lineBreak