"use strict";
/**
 * Data models for EVIF Memory API.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.GraphQueryType = exports.Modality = exports.MemoryType = void 0;
var MemoryType;
(function (MemoryType) {
    MemoryType["PROFILE"] = "profile";
    MemoryType["EVENT"] = "event";
    MemoryType["KNOWLEDGE"] = "knowledge";
    MemoryType["BEHAVIOR"] = "behavior";
    MemoryType["SKILL"] = "skill";
    MemoryType["TOOL"] = "tool";
    MemoryType["CONVERSATION"] = "conversation";
    MemoryType["DOCUMENT"] = "document";
})(MemoryType || (exports.MemoryType = MemoryType = {}));
var Modality;
(function (Modality) {
    Modality["TEXT"] = "text";
    Modality["CONVERSATION"] = "conversation";
    Modality["DOCUMENT"] = "document";
    Modality["IMAGE"] = "image";
    Modality["VIDEO"] = "video";
    Modality["AUDIO"] = "audio";
})(Modality || (exports.Modality = Modality = {}));
var GraphQueryType;
(function (GraphQueryType) {
    GraphQueryType["CAUSAL_CHAIN"] = "causal_chain";
    GraphQueryType["TIMELINE"] = "timeline";
    GraphQueryType["TEMPORAL_BFS"] = "temporal_bfs";
    GraphQueryType["TEMPORAL_PATH"] = "temporal_path";
})(GraphQueryType || (exports.GraphQueryType = GraphQueryType = {}));
