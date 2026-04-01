"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.EvifClient = void 0;
class EvifClient {
    constructor(config) {
        if (typeof config === 'string') {
            this.baseUrl = config;
            this.timeout = 30000;
        }
        else {
            this.baseUrl = config?.baseUrl ?? 'http://localhost:8080';
            this.timeout = config?.timeout ?? 30000;
            this.apiKey = config?.apiKey;
        }
    }
    async request(method, path, body) {
        const headers = { 'Content-Type': 'application/json' };
        if (this.apiKey) {
            headers['Authorization'] = `Bearer ${this.apiKey}`;
        }
        const response = await fetch(`${this.baseUrl}${path}`, {
            method,
            headers,
            body: body ? JSON.stringify(body) : undefined,
            signal: AbortSignal.timeout(this.timeout),
        });
        if (!response.ok) {
            const text = await response.text().catch(() => '');
            throw new Error(`EVIF error ${response.status}: ${text}`);
        }
        return response.json();
    }
    // ===== File Operations =====
    async ls(path) {
        const result = await this.request('POST', '/api/v1/fs/ls', { path });
        if (Array.isArray(result))
            return result;
        return result.entries ?? result;
    }
    async cat(path, offset, size) {
        const body = { path };
        if (offset !== undefined)
            body.offset = offset;
        if (size !== undefined)
            body.size = size;
        const result = await this.request('POST', '/api/v1/fs/read', body);
        return result.data ?? result.content ?? result;
    }
    async write(path, content, offset) {
        const body = { path, data: content };
        if (offset !== undefined)
            body.offset = offset;
        const result = await this.request('POST', '/api/v1/fs/write', body);
        return result.bytes_written ?? result.size ?? result;
    }
    async mkdir(path, mode) {
        const body = { path };
        if (mode !== undefined)
            body.perm = mode;
        const result = await this.request('POST', '/api/v1/fs/mkdir', body);
        return result.success ?? result.ok ?? true;
    }
    async rm(path, recursive) {
        const body = { path };
        if (recursive !== undefined)
            body.recursive = recursive;
        const result = await this.request('DELETE', '/api/v1/fs/remove', body);
        return result.success ?? result.ok ?? true;
    }
    async stat(path) {
        const result = await this.request('POST', '/api/v1/fs/stat', { path });
        return result;
    }
    async mv(oldPath, newPath) {
        const result = await this.request('POST', '/api/v1/fs/rename', {
            old_path: oldPath,
            new_path: newPath,
        });
        return result.success ?? result.ok ?? true;
    }
    async cp(src, dst) {
        const result = await this.request('POST', '/api/v1/fs/copy', {
            src_path: src,
            dst_path: dst,
        });
        return result.success ?? result.ok ?? true;
    }
    async grep(path, pattern, recursive) {
        const body = { path, pattern };
        if (recursive !== undefined)
            body.recursive = recursive;
        const result = await this.request('POST', '/api/v1/fs/grep', body);
        return result.matches ?? result.lines ?? result;
    }
    async create(path) {
        const result = await this.request('POST', '/api/v1/fs/create', { path });
        return result.success ?? result.ok ?? true;
    }
    // ===== Mount Operations =====
    async mount(plugin, path, options) {
        const body = { plugin, path };
        if (options)
            body.options = options;
        const result = await this.request('POST', '/api/v1/mount/add', body);
        return result.success ?? result.ok ?? true;
    }
    async unmount(path) {
        const result = await this.request('POST', '/api/v1/mount/remove', { path });
        return result.success ?? result.ok ?? true;
    }
    async mounts() {
        const result = await this.request('GET', '/api/v1/mount/list');
        if (Array.isArray(result))
            return result;
        return result.mounts ?? result;
    }
    // ===== Health =====
    async health() {
        const result = await this.request('GET', '/health');
        return result;
    }
    // ===== Context API =====
    async contextRead(path) {
        return this.cat(`/context/${path}`);
    }
    async contextWrite(path, content) {
        return this.write(`/context/${path}`, content);
    }
    async contextList(layer) {
        return this.ls(layer ? `/context/${layer}` : '/context');
    }
    async contextCurrent() {
        return this.contextRead('L0/current');
    }
    async contextUpdateCurrent(context) {
        return this.contextWrite('L0/current', context);
    }
    async contextDecisions() {
        return this.contextRead('L1/decisions.md');
    }
    async contextAddDecision(decision) {
        const existing = await this.contextDecisions();
        const updated = existing + '\n' + decision;
        return this.contextWrite('L1/decisions.md', updated);
    }
    async contextRecentOps() {
        const raw = await this.contextRead('L1/recent_ops.json');
        return JSON.parse(raw);
    }
    async contextSearch(query, layer) {
        const basePath = layer ? `/context/${layer}` : '/context';
        const result = await this.grep(basePath, query, true);
        return result;
    }
    async contextMeta() {
        const raw = await this.contextRead('L0/meta.json');
        return JSON.parse(raw);
    }
    async contextKnowledge(name) {
        return this.contextRead(`L2/${name}`);
    }
    async contextAddKnowledge(name, content) {
        return this.contextWrite(`L2/${name}`, content);
    }
    // ===== Skill API =====
    async skillDiscover() {
        const entries = await this.ls('/skills');
        return entries.filter((e) => e.is_dir).map((e) => e.name);
    }
    async skillRead(name) {
        return this.cat(`/skills/${name}/SKILL.md`);
    }
    async skillExecute(name, input) {
        await this.write(`/skills/${name}/input`, input);
        return this.cat(`/skills/${name}/output`);
    }
    async skillRegister(name, skillMd) {
        const created = await this.mkdir(`/skills/${name}`);
        if (!created)
            return false;
        await this.write(`/skills/${name}/SKILL.md`, skillMd);
        return true;
    }
    async skillMatch(query) {
        const skills = await this.skillDiscover();
        for (const name of skills) {
            try {
                const content = await this.skillRead(name);
                const triggerMatch = content.match(/##\s*Triggers?\s*\n([\s\S]*?)(?=\n##|\n*$)/i);
                if (triggerMatch) {
                    const triggers = triggerMatch[1]
                        .split('\n')
                        .map((line) => line.replace(/^[-*]\s*/, '').trim())
                        .filter((line) => line.length > 0);
                    for (const trigger of triggers) {
                        if (query.toLowerCase().includes(trigger.toLowerCase())) {
                            return name;
                        }
                    }
                }
            }
            catch {
                // Skip skills that cannot be read
                continue;
            }
        }
        return null;
    }
    async skillRemove(name) {
        return this.rm(`/skills/${name}`, true);
    }
}
exports.EvifClient = EvifClient;
//# sourceMappingURL=client.js.map