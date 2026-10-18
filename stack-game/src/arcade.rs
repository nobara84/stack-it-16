//! Optional Kitchen-Radio arcade-session integration.
//!
//! The game remains completely standalone when the documented environment
//! variables are absent.  This module deliberately uses only the standard
//! library so the arcade bridge does not become a gameplay dependency.

use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::io::{self, Read, Write};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

pub const NEXT_BUTTON: (f32, f32, f32, f32) = (24.0, 143.0, 272.0, 31.0);
pub const RETURN_BUTTON: (f32, f32, f32, f32) = (24.0, 181.0, 272.0, 31.0);

const PROTOCOL_VERSION: u32 = 1;
const MAX_ATTEMPTS: u8 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArcadeConfig {
    pub socket_path: String,
    pub session_id: String,
    pub attempts: u8,
    pub currency: String,
}

/// Parse the four documented variables.  An entirely absent configuration is
/// the normal standalone mode; a partial or invalid configuration is rejected
/// rather than silently starting a misleading arcade session.
pub fn config_from_values(
    socket_path: Option<&str>,
    session_id: Option<&str>,
    attempts: Option<&str>,
    currency: Option<&str>,
) -> Result<Option<ArcadeConfig>, String> {
    if socket_path.is_none() && session_id.is_none() && attempts.is_none() && currency.is_none() {
        return Ok(None);
    }

    let socket_path = socket_path
        .filter(|value| !value.is_empty())
        .ok_or("STACK_IT_16_ARCADE_SOCKET fehlt")?;
    let session_id = session_id
        .filter(|value| !value.is_empty())
        .ok_or("STACK_IT_16_ARCADE_SESSION_ID fehlt")?;
    let attempts = attempts
        .ok_or("STACK_IT_16_ARCADE_ATTEMPTS fehlt")?
        .parse::<u8>()
        .map_err(|_| "STACK_IT_16_ARCADE_ATTEMPTS ist keine Zahl")?;
    if attempts == 0 || attempts > MAX_ATTEMPTS {
        return Err(format!(
            "STACK_IT_16_ARCADE_ATTEMPTS muss zwischen 1 und {MAX_ATTEMPTS} liegen"
        ));
    }
    let currency = currency.ok_or("STACK_IT_16_ARCADE_CURRENCY fehlt")?;
    if !matches!(currency, "rings" | "bananas") {
        return Err("STACK_IT_16_ARCADE_CURRENCY muss rings oder bananas sein".to_owned());
    }

    Ok(Some(ArcadeConfig {
        socket_path: socket_path.to_owned(),
        session_id: session_id.to_owned(),
        attempts,
        currency: currency.to_owned(),
    }))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArcadeEvent {
    SessionStarted {
        session_id: String,
    },
    AttemptStarted {
        session_id: String,
        attempt_id: u8,
    },
    Milestone {
        session_id: String,
        attempt_id: u8,
        blocks: u32,
        reward: u32,
        currency: String,
        idempotency_key: String,
    },
    AttemptFinished {
        session_id: String,
        attempt_id: u8,
        blocks: u32,
        outcome: &'static str,
    },
    SessionCompleted {
        session_id: String,
    },
    VoluntaryExit {
        session_id: String,
        reason: &'static str,
    },
}

impl ArcadeEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::SessionStarted { .. } => "session_started",
            Self::AttemptStarted { .. } => "attempt_started",
            Self::Milestone { .. } => "milestone",
            Self::AttemptFinished { .. } => "attempt_finished",
            Self::SessionCompleted { .. } => "session_completed",
            Self::VoluntaryExit { .. } => "voluntary_exit",
        }
    }

    fn session_id(&self) -> &str {
        match self {
            Self::SessionStarted { session_id }
            | Self::AttemptStarted { session_id, .. }
            | Self::Milestone { session_id, .. }
            | Self::AttemptFinished { session_id, .. }
            | Self::SessionCompleted { session_id }
            | Self::VoluntaryExit { session_id, .. } => session_id,
        }
    }

    pub fn to_json(&self) -> String {
        let prefix = format!(
            "{{\"protocol\":{PROTOCOL_VERSION},\"type\":\"{}\",\"session_id\":{}",
            self.event_type(),
            json_string(self.session_id())
        );
        match self {
            Self::SessionStarted { .. } | Self::SessionCompleted { .. } => format!("{prefix}}}"),
            Self::AttemptStarted { attempt_id, .. } => {
                format!("{prefix},\"attempt_id\":{attempt_id}}}")
            }
            Self::Milestone {
                attempt_id,
                blocks,
                reward,
                currency,
                idempotency_key,
                ..
            } => format!(
                "{prefix},\"attempt_id\":{attempt_id},\"blocks\":{blocks},\"reward\":{reward},\"currency\":{},\"idempotency_key\":{}}}",
                json_string(currency),
                json_string(idempotency_key)
            ),
            Self::AttemptFinished {
                attempt_id,
                blocks,
                outcome,
                ..
            } => format!(
                "{prefix},\"attempt_id\":{attempt_id},\"blocks\":{blocks},\"outcome\":{}}}",
                json_string(outcome)
            ),
            Self::VoluntaryExit { reason, .. } => {
                format!("{prefix},\"reason\":{}}}", json_string(reason))
            }
        }
    }
}

#[derive(Debug)]
pub struct ArcadeSession {
    config: ArcadeConfig,
    attempts_remaining: u8,
    next_attempt_id: u8,
    current_attempt: Option<u8>,
    session_started: bool,
    completed: bool,
    milestones: BTreeSet<(u8, u32)>,
}

impl ArcadeSession {
    pub fn new(config: ArcadeConfig) -> Self {
        Self {
            attempts_remaining: config.attempts,
            config,
            next_attempt_id: 1,
            current_attempt: None,
            session_started: false,
            completed: false,
            milestones: BTreeSet::new(),
        }
    }

    pub fn attempts_remaining(&self) -> u8 {
        self.attempts_remaining
    }

    #[cfg(test)]
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    #[cfg(test)]
    pub fn milestone_count(&self) -> usize {
        self.milestones.len()
    }

    pub fn start_attempt(&mut self) -> Vec<ArcadeEvent> {
        if self.completed || self.current_attempt.is_some() || self.attempts_remaining == 0 {
            return Vec::new();
        }
        let attempt_id = self.next_attempt_id;
        self.next_attempt_id = self.next_attempt_id.saturating_add(1);
        self.current_attempt = Some(attempt_id);
        let mut events = Vec::with_capacity(2);
        if !self.session_started {
            self.session_started = true;
            events.push(ArcadeEvent::SessionStarted {
                session_id: self.config.session_id.clone(),
            });
        }
        events.push(ArcadeEvent::AttemptStarted {
            session_id: self.config.session_id.clone(),
            attempt_id,
        });
        events
    }

    pub fn successful_placement(&mut self, blocks: u32) -> Option<ArcadeEvent> {
        let attempt_id = self.current_attempt?;
        let reward = milestone_reward(blocks)?;
        if !self.milestones.insert((attempt_id, blocks)) {
            return None;
        }
        Some(ArcadeEvent::Milestone {
            session_id: self.config.session_id.clone(),
            attempt_id,
            blocks,
            reward,
            currency: self.config.currency.clone(),
            idempotency_key: idempotency_key(&self.config.session_id, attempt_id, blocks),
        })
    }

    pub fn finish_attempt(&mut self, blocks: u32) -> Vec<ArcadeEvent> {
        let Some(attempt_id) = self.current_attempt.take() else {
            return Vec::new();
        };
        self.attempts_remaining = self.attempts_remaining.saturating_sub(1);
        let mut events = vec![ArcadeEvent::AttemptFinished {
            session_id: self.config.session_id.clone(),
            attempt_id,
            blocks,
            outcome: "game_over",
        }];
        if self.attempts_remaining == 0 {
            self.completed = true;
            events.push(ArcadeEvent::SessionCompleted {
                session_id: self.config.session_id.clone(),
            });
        }
        events
    }

    pub fn voluntary_exit(&mut self) -> Option<ArcadeEvent> {
        if self.completed {
            return None;
        }
        self.completed = true;
        Some(ArcadeEvent::VoluntaryExit {
            session_id: self.config.session_id.clone(),
            reason: "user",
        })
    }
}

/// Return the reward for the exact successful placement count.  The
/// foundation is represented by zero placements in stack-core and therefore
/// never reaches this table.
pub fn milestone_reward(blocks: u32) -> Option<u32> {
    match blocks {
        20 | 40 | 60 | 80 | 100 => Some(20),
        value if value >= 110 && value % 10 == 0 => Some(10),
        _ => None,
    }
}

pub fn idempotency_key(session_id: &str, attempt_id: u8, blocks: u32) -> String {
    format!("{session_id}:{attempt_id}:{blocks}")
}

pub struct ArcadeClient {
    session: Option<ArcadeSession>,
    #[cfg(unix)]
    stream: Option<UnixStream>,
    ipc_failed: bool,
}

impl ArcadeClient {
    pub fn from_env() -> Self {
        let values = [
            env::var("STACK_IT_16_ARCADE_SOCKET").ok(),
            env::var("STACK_IT_16_ARCADE_SESSION_ID").ok(),
            env::var("STACK_IT_16_ARCADE_ATTEMPTS").ok(),
            env::var("STACK_IT_16_ARCADE_CURRENCY").ok(),
        ];
        match config_from_values(
            values[0].as_deref(),
            values[1].as_deref(),
            values[2].as_deref(),
            values[3].as_deref(),
        ) {
            Ok(Some(config)) => Self::new(config),
            Ok(None) => Self::standalone(),
            Err(error) => {
                eprintln!("[stack-it-16] Arcade-Umgebung ungültig: {error}");
                Self::standalone()
            }
        }
    }

    pub fn standalone() -> Self {
        Self {
            session: None,
            #[cfg(unix)]
            stream: None,
            ipc_failed: false,
        }
    }

    pub fn new(config: ArcadeConfig) -> Self {
        Self {
            session: Some(ArcadeSession::new(config)),
            #[cfg(unix)]
            stream: None,
            ipc_failed: false,
        }
    }

    pub fn enabled(&self) -> bool {
        self.session.is_some()
    }

    pub fn attempts_remaining(&self) -> Option<u8> {
        self.session.as_ref().map(ArcadeSession::attempts_remaining)
    }

    #[cfg(test)]
    pub fn ipc_failed(&self) -> bool {
        self.ipc_failed
    }

    pub fn begin_attempt(&mut self) {
        let events = self
            .session
            .as_mut()
            .map(ArcadeSession::start_attempt)
            .unwrap_or_default();
        self.send_events(events);
    }

    pub fn successful_placement(&mut self, blocks: u32) {
        let event = self
            .session
            .as_mut()
            .and_then(|session| session.successful_placement(blocks));
        if let Some(event) = event {
            self.send_event(event);
        }
    }

    pub fn finish_attempt(&mut self, blocks: u32) {
        let events = self
            .session
            .as_mut()
            .map(|session| session.finish_attempt(blocks))
            .unwrap_or_default();
        self.send_events(events);
    }

    pub fn voluntary_exit(&mut self) {
        let event = self
            .session
            .as_mut()
            .and_then(ArcadeSession::voluntary_exit);
        if let Some(event) = event {
            self.send_event(event);
        }
    }

    fn send_events(&mut self, events: Vec<ArcadeEvent>) {
        for event in events {
            self.send_event(event);
        }
    }

    fn send_event(&mut self, event: ArcadeEvent) {
        if self.ipc_failed || self.session.is_none() {
            return;
        }
        let message = event.to_json();
        if let Err(error) = self.send_line(&message) {
            self.ipc_failed = true;
            eprintln!(
                "[stack-it-16] Arcade-IPC fehlgeschlagen ({}): {error}",
                event.event_type()
            );
        }
    }

    #[cfg(unix)]
    fn send_line(&mut self, message: &str) -> Result<(), IpcError> {
        if self.stream.is_none() {
            let config = self
                .session
                .as_ref()
                .expect("ArcadeClient stream requires a session");
            let stream = UnixStream::connect(&config.config.socket_path)?;
            stream.set_read_timeout(Some(Duration::from_millis(250)))?;
            stream.set_write_timeout(Some(Duration::from_millis(250)))?;
            self.stream = Some(stream);
        }
        let stream = self.stream.as_mut().expect("stream initialized");
        stream.write_all(message.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;

        let mut response = Vec::with_capacity(64);
        let mut byte = [0_u8; 1];
        while response.len() < 512 {
            let count = stream.read(&mut byte)?;
            if count == 0 {
                return Err(IpcError::InvalidAck("bridge closed the socket".to_owned()));
            }
            if byte[0] == b'\n' {
                break;
            }
            response.push(byte[0]);
        }
        if response.len() == 512 || !acknowledged(&response) {
            return Err(IpcError::InvalidAck(
                String::from_utf8_lossy(&response).into_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn send_line(&mut self, _message: &str) -> Result<(), IpcError> {
        Err(IpcError::UnsupportedPlatform)
    }
}

#[derive(Debug)]
enum IpcError {
    Io(io::Error),
    InvalidAck(String),
    #[cfg(not(unix))]
    UnsupportedPlatform,
}

impl fmt::Display for IpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::InvalidAck(response) => write!(formatter, "ungültige Bestätigung: {response}"),
            #[cfg(not(unix))]
            Self::UnsupportedPlatform => {
                formatter.write_str("Unix-Sockets werden hier nicht unterstützt")
            }
        }
    }
}

impl From<io::Error> for IpcError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn acknowledged(response: &[u8]) -> bool {
    let text = String::from_utf8_lossy(response);
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    compact.contains("\"protocol\":1") && compact.contains("\"ok\":true")
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(escaped, "\\u{:04x}", character as u32);
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ArcadeConfig {
        ArcadeConfig {
            socket_path: "/tmp/stack-it-16-test.sock".to_owned(),
            session_id: "session-test".to_owned(),
            attempts: 3,
            currency: "rings".to_owned(),
        }
    }

    #[test]
    fn missing_arcade_environment_is_standalone() {
        assert_eq!(config_from_values(None, None, None, None).unwrap(), None);
        assert!(config_from_values(Some("/tmp/socket"), None, Some("3"), Some("rings")).is_err());
    }

    #[test]
    fn arcade_configuration_accepts_the_documented_attempt_count_and_currency() {
        assert!(
            config_from_values(Some("/tmp/socket"), Some("id"), Some("3"), Some("bananas"))
                .unwrap()
                .is_some()
        );
        assert!(
            config_from_values(Some("/tmp/socket"), Some("id"), Some("4"), Some("rings")).is_err()
        );
        assert!(
            config_from_values(Some("/tmp/socket"), Some("id"), Some("2"), Some("rings"))
                .unwrap()
                .is_some()
        );
        assert!(
            config_from_values(Some("/tmp/socket"), Some("id"), Some("3"), Some("coins")).is_err()
        );
    }

    #[test]
    fn milestone_table_excludes_foundation_and_follows_protocol() {
        assert_eq!(milestone_reward(0), None);
        assert_eq!(milestone_reward(19), None);
        assert_eq!(milestone_reward(20), Some(20));
        assert_eq!(milestone_reward(100), Some(20));
        assert_eq!(milestone_reward(109), None);
        assert_eq!(milestone_reward(110), Some(10));
        assert_eq!(milestone_reward(500), Some(10));
    }

    #[test]
    fn session_has_three_attempts_and_completes_after_the_third() {
        let mut session = ArcadeSession::new(config());
        assert_eq!(session.attempts_remaining(), 3);
        assert_eq!(session.start_attempt().len(), 2);
        assert_eq!(session.finish_attempt(0).len(), 1);
        assert_eq!(session.attempts_remaining(), 2);
        assert_eq!(session.start_attempt().len(), 1);
        session.finish_attempt(0);
        session.start_attempt();
        let final_events = session.finish_attempt(0);
        assert_eq!(final_events.len(), 2);
        assert!(session.is_completed());
        assert_eq!(session.attempts_remaining(), 0);
    }

    #[test]
    fn milestones_are_idempotent_per_attempt() {
        let mut session = ArcadeSession::new(config());
        session.start_attempt();
        assert!(session.successful_placement(20).is_some());
        assert!(session.successful_placement(20).is_none());
        assert_eq!(session.milestone_count(), 1);
        session.finish_attempt(20);
        session.start_attempt();
        assert!(session.successful_placement(20).is_some());
        assert_eq!(session.milestone_count(), 2);
    }

    #[test]
    fn replay_does_not_create_rewards_without_a_successful_placement_event() {
        let mut session = ArcadeSession::new(config());
        session.start_attempt();
        assert_eq!(session.milestone_count(), 0);
        session.finish_attempt(20);
        assert_eq!(session.milestone_count(), 0);
    }

    #[test]
    fn voluntary_exit_is_emitted_only_before_completion() {
        let mut session = ArcadeSession::new(config());
        session.start_attempt();
        assert!(matches!(
            session.voluntary_exit(),
            Some(ArcadeEvent::VoluntaryExit { .. })
        ));
        assert!(session.voluntary_exit().is_none());
    }

    #[test]
    fn completion_event_is_not_a_voluntary_exit() {
        let mut session = ArcadeSession::new(config());
        for _ in 0..3 {
            session.start_attempt();
            session.finish_attempt(0);
        }
        assert!(session.is_completed());
        assert!(session.voluntary_exit().is_none());
    }

    #[test]
    fn event_json_has_protocol_fields_and_idempotency_key() {
        let event = ArcadeEvent::Milestone {
            session_id: "s".to_owned(),
            attempt_id: 1,
            blocks: 20,
            reward: 20,
            currency: "rings".to_owned(),
            idempotency_key: "s:1:20".to_owned(),
        };
        let json = event.to_json();
        assert!(json.contains("\"protocol\":1"));
        assert!(json.contains("\"type\":\"milestone\""));
        assert!(json.contains("\"idempotency_key\":\"s:1:20\""));
    }

    #[test]
    fn invalid_ack_is_not_treated_as_a_success() {
        assert!(!acknowledged(br#"{"protocol":1,"ok":false}"#));
        assert!(!acknowledged(br#"{"protocol":2,"ok":true}"#));
        assert!(acknowledged(br#" { "protocol": 1, "ok": true } "#));
    }

    #[cfg(unix)]
    #[test]
    fn ipc_error_is_recorded_and_cannot_book_a_reward() {
        let mut client = ArcadeClient::new(config());
        client.begin_attempt();
        client.successful_placement(20);
        assert!(client.ipc_failed());
    }
}
