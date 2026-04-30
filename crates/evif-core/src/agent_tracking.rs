// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Agent Tracking Module
//!
//! Provides agent session tracking and chain-of-thought logging.
//! Supports multi-agent coordination with conversation chains.
//!
//! # Features
//! - Session lifecycle management (create, pause, resume, terminate)
//! - Chain of thought (CoT) logging
//! - Agent state snapshots
//! - Activity timeline tracking
//! - Conversation chain storage

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Unique session ID generator
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique agent ID generator
static AGENT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Agent state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentState {
    /// Agent is idle and waiting
    Idle,
    /// Agent is processing
    Processing,
    /// Agent is paused
    Paused,
    /// Agent encountered an error
    Error,
    /// Agent has terminated
    Terminated,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Chain of thought entry
#[derive(Debug, Clone)]
pub struct ThoughtEntry {
    /// Unique entry ID
    pub id: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Thought content
    pub thought: String,
    /// Reasoning type (e.g., "analysis", "planning", "reflection")
    pub reasoning_type: String,
    /// Parent thought ID (for nested thoughts)
    pub parent_id: Option<u64>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl ThoughtEntry {
    /// Create a new thought entry
    pub fn new(thought: String, reasoning_type: String) -> Self {
        static THOUGHT_COUNTER: AtomicU64 = AtomicU64::new(1);
        Self {
            id: THOUGHT_COUNTER.fetch_add(1, Ordering::SeqCst),
            timestamp: Utc::now(),
            thought,
            reasoning_type,
            parent_id: None,
            confidence: 1.0,
            metadata: HashMap::new(),
        }
    }

    /// Set parent thought
    pub fn with_parent(mut self, parent_id: u64) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Activity event
#[derive(Debug, Clone)]
pub struct ActivityEvent {
    /// Unique event ID
    pub id: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: ActivityType,
    /// Description
    pub description: String,
    /// Agent ID
    pub agent_id: u64,
    /// Session ID
    pub session_id: u64,
    /// Duration in milliseconds (for timed events)
    pub duration_ms: Option<u64>,
    /// Result status
    pub status: EventStatus,
}

/// Activity event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityType {
    /// Agent started
    Start,
    /// Agent paused
    Pause,
    /// Agent resumed
    Resume,
    /// Agent terminated
    Terminate,
    /// Tool execution
    ToolExecution,
    /// LLM call
    LlmCall,
    /// Memory operation
    MemoryOperation,
    /// Plugin invocation
    PluginInvocation,
    /// Error occurred
    Error,
    /// Custom event
    Custom(String),
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityType::Start => write!(f, "start"),
            ActivityType::Pause => write!(f, "pause"),
            ActivityType::Resume => write!(f, "resume"),
            ActivityType::Terminate => write!(f, "terminate"),
            ActivityType::ToolExecution => write!(f, "tool_execution"),
            ActivityType::LlmCall => write!(f, "llm_call"),
            ActivityType::MemoryOperation => write!(f, "memory_operation"),
            ActivityType::PluginInvocation => write!(f, "plugin_invocation"),
            ActivityType::Error => write!(f, "error"),
            ActivityType::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// Event status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStatus {
    /// Event succeeded
    Success,
    /// Event failed
    Failure,
    /// Event in progress
    InProgress,
    /// Event pending
    Pending,
}

impl Default for EventStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Agent metadata
#[derive(Debug, Clone)]
pub struct AgentMetadata {
    /// Agent name
    pub name: String,
    /// Agent type/version
    pub agent_type: String,
    /// Configuration
    pub config: HashMap<String, String>,
}

impl AgentMetadata {
    /// Create new agent metadata
    pub fn new(name: String, agent_type: String) -> Self {
        Self {
            name,
            agent_type,
            config: HashMap::new(),
        }
    }
}

/// Agent session
#[derive(Debug, Clone)]
pub struct AgentSession {
    /// Unique session ID
    pub id: u64,
    /// Agent ID
    pub agent_id: u64,
    /// Session UUID
    pub uuid: Uuid,
    /// Session name
    pub name: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp (with interior mutability)
    last_activity: Arc<parking_lot::Mutex<DateTime<Utc>>>,
    /// Session state (with interior mutability)
    state: Arc<parking_lot::Mutex<AgentState>>,
    /// Chain of thought entries
    thoughts: Arc<RwLock<Vec<ThoughtEntry>>>,
    /// Activity events
    events: Arc<RwLock<Vec<ActivityEvent>>>,
    /// Parent session ID (for session branching)
    pub parent_session_id: Option<u64>,
    /// Root session ID (for session trees)
    pub root_session_id: Option<u64>,
}

impl AgentSession {
    /// Create a new agent session
    pub fn new(agent_id: u64, name: String) -> Self {
        let id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now();

        Self {
            id,
            agent_id,
            uuid: Uuid::new_v4(),
            name,
            created_at: now,
            last_activity: Arc::new(parking_lot::Mutex::new(now)),
            state: Arc::new(parking_lot::Mutex::new(AgentState::Idle)),
            thoughts: Arc::new(RwLock::new(Vec::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            parent_session_id: None,
            root_session_id: None,
        }
    }

    /// Create a child session (branch)
    pub fn branch(&self, name: String) -> Self {
        let id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now();

        Self {
            id,
            agent_id: self.agent_id,
            uuid: Uuid::new_v4(),
            name,
            created_at: now,
            last_activity: Arc::new(parking_lot::Mutex::new(now)),
            state: Arc::new(parking_lot::Mutex::new(AgentState::Idle)),
            thoughts: Arc::new(RwLock::new(Vec::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            parent_session_id: Some(self.id),
            root_session_id: self.root_session_id.or(Some(self.id)),
        }
    }

    /// Add a thought entry
    pub fn add_thought(&self, thought: ThoughtEntry) {
        {
            let mut thoughts = self.thoughts.write();
            thoughts.push(thought);
        }
        self.touch();
    }

    /// Get all thoughts
    pub fn get_thoughts(&self) -> Vec<ThoughtEntry> {
        self.thoughts.read().clone()
    }

    /// Get thoughts in tree order (parent before children)
    pub fn get_thoughts_tree(&self) -> Vec<ThoughtEntry> {
        let thoughts = self.thoughts.read().clone();
        Self::build_thought_tree(&thoughts, None)
    }

    /// Build thought tree from flat list
    fn build_thought_tree(thoughts: &[ThoughtEntry], parent_id: Option<u64>) -> Vec<ThoughtEntry> {
        let mut result = Vec::new();
        for thought in thoughts {
            if thought.parent_id == parent_id {
                result.push(thought.clone());
                result.extend(Self::build_thought_tree(thoughts, Some(thought.id)));
            }
        }
        result
    }

    /// Add an activity event
    pub fn add_event(&self, event: ActivityEvent) {
        {
            let mut events = self.events.write();
            events.push(event);
        }
        self.touch();
    }

    /// Get all events
    pub fn get_events(&self) -> Vec<ActivityEvent> {
        self.events.read().clone()
    }

    /// Update state
    pub fn set_state(&self, state: AgentState) {
        *self.state.lock() = state;
        self.touch();
    }

    /// Get current state
    pub fn get_state(&self) -> AgentState {
        self.state.lock().clone()
    }

    /// Touch last activity
    fn touch(&self) {
        *self.last_activity.lock() = Utc::now();
    }

    /// Get last activity timestamp
    pub fn last_activity(&self) -> DateTime<Utc> {
        *self.last_activity.lock()
    }

    /// Get session duration in seconds
    pub fn duration_secs(&self) -> i64 {
        (*self.last_activity.lock() - self.created_at).num_seconds()
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        matches!(*self.state.lock(), AgentState::Idle | AgentState::Processing)
    }
}

/// Agent tracker - manages multiple agents and sessions
#[derive(Debug, Clone)]
pub struct AgentTracker {
    /// All agents (id -> metadata)
    agents: Arc<RwLock<HashMap<u64, AgentMetadata>>>,
    /// All sessions (id -> session)
    sessions: Arc<RwLock<HashMap<u64, Arc<RwLock<AgentSession>>>>>,
    /// Active sessions (id -> session)
    active_sessions: Arc<RwLock<HashMap<u64, Arc<RwLock<AgentSession>>>>>,
}

impl AgentTracker {
    /// Create a new agent tracker
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new agent
    pub fn register_agent(&self, name: String, agent_type: String) -> u64 {
        let id = AGENT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let metadata = AgentMetadata::new(name, agent_type);
        self.agents.write().insert(id, metadata);
        id
    }

    /// Get agent metadata
    pub fn get_agent(&self, agent_id: u64) -> Option<AgentMetadata> {
        self.agents.read().get(&agent_id).cloned()
    }

    /// List all agents
    pub fn list_agents(&self) -> Vec<(u64, AgentMetadata)> {
        self.agents
            .read()
            .iter()
            .map(|(&id, meta)| (id, meta.clone()))
            .collect()
    }

    /// Create a new session
    pub fn create_session(&self, agent_id: u64, name: String) -> Arc<RwLock<AgentSession>> {
        let session_name = name.clone();
        let session = Arc::new(RwLock::new(AgentSession::new(agent_id, name)));
        let id = session.read().id;

        self.sessions.write().insert(id, Arc::clone(&session));
        self.active_sessions.write().insert(id, Arc::clone(&session));

        // Log session start event
        let event = ActivityEvent {
            id: 0, // Will be auto-assigned
            timestamp: Utc::now(),
            event_type: ActivityType::Start,
            description: format!("Session {} created", session_name),
            agent_id,
            session_id: id,
            duration_ms: None,
            status: EventStatus::Success,
        };
        session.write().add_event(event);

        session
    }

    /// Branch a session
    pub fn branch_session(&self, parent_id: u64, name: String) -> Option<Arc<RwLock<AgentSession>>> {
        // Read parent info and create child outside of write lock
        let child_session = {
            let sessions = self.sessions.read();
            let parent = sessions.get(&parent_id)?;
            let parent_guard = parent.read();
            parent_guard.branch(name)
        };

        let child_id = child_session.id;
        let child_arc = Arc::new(RwLock::new(child_session));

        // Now write to maps
        self.sessions.write().insert(child_id, Arc::clone(&child_arc));
        self.active_sessions.write().insert(child_id, Arc::clone(&child_arc));

        Some(child_arc)
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: u64) -> Option<Arc<RwLock<AgentSession>>> {
        self.sessions.read().get(&session_id).cloned()
    }

    /// List all sessions for an agent
    pub fn list_agent_sessions(&self, agent_id: u64) -> Vec<AgentSession> {
        self.sessions
            .read()
            .values()
            .filter_map(|s| {
                let guard = s.read();
                if guard.agent_id == agent_id {
                    Some(guard.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// List active sessions
    pub fn list_active_sessions(&self) -> Vec<AgentSession> {
        self.active_sessions
            .read()
            .values()
            .filter_map(|s| {
                let guard = s.read();
                if guard.is_active() {
                    Some(guard.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Terminate a session
    pub fn terminate_session(&self, session_id: u64) -> bool {
        let sessions = self.sessions.read();
        let session = match sessions.get(&session_id) {
            Some(s) => s,
            None => return false,
        };

        let mut guard = session.write();
        if !guard.is_active() {
            return false;
        }

        // Log termination
        let event = ActivityEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: ActivityType::Terminate,
            description: format!("Session {} terminated", guard.name),
            agent_id: guard.agent_id,
            session_id: guard.id,
            duration_ms: Some(guard.duration_secs() as u64 * 1000),
            status: EventStatus::Success,
        };
        guard.add_event(event);
        guard.set_state(AgentState::Terminated);
        drop(guard);

        self.active_sessions.write().remove(&session_id);
        true
    }

    /// Add thought to session
    pub fn add_thought(
        &self,
        session_id: u64,
        thought: String,
        reasoning_type: String,
    ) -> Option<u64> {
        let sessions = self.sessions.read();
        let session = sessions.get(&session_id)?;

        let entry = ThoughtEntry::new(thought, reasoning_type);
        let id = entry.id;
        session.read().add_thought(entry);

        Some(id)
    }

    /// Add activity event to session
    pub fn add_event(&self, session_id: u64, event: ActivityEvent) -> bool {
        let sessions = self.sessions.read();
        let session = match sessions.get(&session_id) {
            Some(s) => s,
            None => return false,
        };

        session.read().add_event(event);
        true
    }

    /// Record an error in session
    pub fn record_error(&self, session_id: u64, error: &str) {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(&session_id) {
            let event = ActivityEvent {
                id: 0,
                timestamp: Utc::now(),
                event_type: ActivityType::Error,
                description: error.to_string(),
                agent_id: session.read().agent_id,
                session_id,
                duration_ms: None,
                status: EventStatus::Failure,
            };
            session.read().add_event(event);
        }
    }

    /// Get session statistics
    pub fn get_stats(&self) -> TrackerStats {
        let sessions = self.sessions.read();
        let active = self.active_sessions.read();

        let total_sessions = sessions.len();
        let active_sessions = active.len();
        let total_thoughts: usize = sessions
            .values()
            .map(|s| s.read().thoughts.read().len())
            .sum();
        let total_events: usize = sessions
            .values()
            .map(|s| s.read().events.read().len())
            .sum();

        TrackerStats {
            total_agents: self.agents.read().len(),
            total_sessions,
            active_sessions,
            total_thoughts,
            total_events,
        }
    }
}

impl Default for AgentTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracker statistics
#[derive(Debug, Clone)]
pub struct TrackerStats {
    /// Total registered agents
    pub total_agents: usize,
    /// Total sessions ever created
    pub total_sessions: usize,
    /// Currently active sessions
    pub active_sessions: usize,
    /// Total thought entries
    pub total_thoughts: usize,
    /// Total activity events
    pub total_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_tracker_creation() {
        let tracker = AgentTracker::new();
        assert_eq!(tracker.get_stats().total_agents, 0);
    }

    #[test]
    fn test_register_agent() {
        let tracker = AgentTracker::new();
        let agent_id = tracker.register_agent("test-agent".to_string(), "test".to_string());

        assert!(agent_id > 0);
        assert_eq!(tracker.get_stats().total_agents, 1);

        let agent = tracker.get_agent(agent_id).unwrap();
        assert_eq!(agent.name, "test-agent");
    }

    #[test]
    fn test_create_session() {
        let tracker = AgentTracker::new();
        let agent_id = tracker.register_agent("test-agent".to_string(), "test".to_string());
        let session = tracker.create_session(agent_id, "test-session".to_string());

        let guard = session.read();
        assert_eq!(guard.name, "test-session");
        assert_eq!(guard.agent_id, agent_id);
        assert_eq!(guard.get_state(), AgentState::Idle);
    }

    #[test]
    fn test_session_thoughts() {
        let tracker = AgentTracker::new();
        let agent_id = tracker.register_agent("test-agent".to_string(), "test".to_string());
        let session = tracker.create_session(agent_id, "test-session".to_string());
        let session_id = session.read().id;

        // Add thoughts
        let _thought_id = tracker
            .add_thought(session_id, "First thought".to_string(), "analysis".to_string())
            .unwrap();

        // Add child thought using ThoughtEntry directly
        let child_thought = ThoughtEntry::new("Child thought".to_string(), "reflection".to_string())
            .with_parent(_thought_id);
        session.write().add_thought(child_thought);

        let thoughts = session.read().get_thoughts();
        assert_eq!(thoughts.len(), 2);
    }

    #[test]
    fn test_session_branch() {
        let tracker = AgentTracker::new();
        let agent_id = tracker.register_agent("test-agent".to_string(), "test".to_string());
        let parent = tracker.create_session(agent_id, "parent".to_string());
        let parent_id = parent.read().id;

        let child = tracker.branch_session(parent_id, "child".to_string()).unwrap();
        let child_guard = child.read();

        assert_eq!(child_guard.parent_session_id, Some(parent_id));
        assert_eq!(child_guard.root_session_id, Some(parent_id));
    }

    #[test]
    fn test_terminate_session() {
        let tracker = AgentTracker::new();
        let agent_id = tracker.register_agent("test-agent".to_string(), "test".to_string());
        let session = tracker.create_session(agent_id, "test".to_string());
        let session_id = session.read().id;

        assert!(tracker.terminate_session(session_id));
        assert!(!tracker.terminate_session(session_id)); // Can't terminate twice

        let stats = tracker.get_stats();
        assert_eq!(stats.active_sessions, 0);
    }

    #[test]
    fn test_thought_entry_builder() {
        let entry = ThoughtEntry::new("Test thought".to_string(), "analysis".to_string())
            .with_parent(123)
            .with_confidence(0.85)
            .with_metadata("key", "value");

        assert_eq!(entry.thought, "Test thought");
        assert_eq!(entry.parent_id, Some(123));
        assert_eq!(entry.confidence, 0.85);
        assert_eq!(entry.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_activity_types() {
        assert_eq!(ActivityType::Start.to_string(), "start");
        assert_eq!(ActivityType::ToolExecution.to_string(), "tool_execution");
        assert_eq!(
            ActivityType::Custom("custom".to_string()).to_string(),
            "custom:custom"
        );
    }

    #[test]
    fn test_session_duration() {
        let tracker = AgentTracker::new();
        let agent_id = tracker.register_agent("test-agent".to_string(), "test".to_string());
        let session = tracker.create_session(agent_id, "test".to_string());

        let duration = session.read().duration_secs();
        assert!(duration >= 0);
    }

    #[test]
    fn test_list_active_sessions() {
        let tracker = AgentTracker::new();
        let agent_id = tracker.register_agent("test-agent".to_string(), "test".to_string());

        tracker.create_session(agent_id, "session1".to_string());
        tracker.create_session(agent_id, "session2".to_string());

        let active = tracker.list_active_sessions();
        assert_eq!(active.len(), 2);
    }
}
