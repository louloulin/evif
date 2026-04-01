import { FileInfo, MountInfo, HealthStatus } from './types';
export interface EvifConfig {
    baseUrl?: string;
    timeout?: number;
    apiKey?: string;
}
export declare class EvifClient {
    private baseUrl;
    private timeout;
    private apiKey?;
    constructor(config?: EvifConfig | string);
    private request;
    ls(path: string): Promise<FileInfo[]>;
    cat(path: string, offset?: number, size?: number): Promise<string>;
    write(path: string, content: string, offset?: number): Promise<number>;
    mkdir(path: string, mode?: number): Promise<boolean>;
    rm(path: string, recursive?: boolean): Promise<boolean>;
    stat(path: string): Promise<FileInfo>;
    mv(oldPath: string, newPath: string): Promise<boolean>;
    cp(src: string, dst: string): Promise<boolean>;
    grep(path: string, pattern: string, recursive?: boolean): Promise<string[]>;
    create(path: string): Promise<boolean>;
    mount(plugin: string, path: string, options?: Record<string, string>): Promise<boolean>;
    unmount(path: string): Promise<boolean>;
    mounts(): Promise<MountInfo[]>;
    health(): Promise<HealthStatus>;
    contextRead(path: string): Promise<string>;
    contextWrite(path: string, content: string): Promise<number>;
    contextList(layer?: string): Promise<FileInfo[]>;
    contextCurrent(): Promise<string>;
    contextUpdateCurrent(context: string): Promise<number>;
    contextDecisions(): Promise<string>;
    contextAddDecision(decision: string): Promise<number>;
    contextRecentOps(): Promise<any[]>;
    contextSearch(query: string, layer?: string): Promise<string[]>;
    contextMeta(): Promise<any>;
    contextKnowledge(name: string): Promise<string>;
    contextAddKnowledge(name: string, content: string): Promise<number>;
    skillDiscover(): Promise<string[]>;
    skillRead(name: string): Promise<string>;
    skillExecute(name: string, input: string): Promise<string>;
    skillRegister(name: string, skillMd: string): Promise<boolean>;
    skillMatch(query: string): Promise<string | null>;
    skillRemove(name: string): Promise<boolean>;
}
