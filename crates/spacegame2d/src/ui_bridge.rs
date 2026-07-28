use std::time::{Duration, Instant};

use spacegame2d_ui_protocol::BridgeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeHealthConfig {
    pub interval: Duration,
    pub missed_ack_threshold: u8,
}
impl Default for BridgeHealthConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(1),
            missed_ack_threshold: 3,
        }
    }
}

#[derive(Debug)]
pub struct UiBridge {
    bridge_id: Option<BridgeId>,
    next_heartbeat: Option<Instant>,
    awaiting: Option<u64>,
    missed: u8,
    config: BridgeHealthConfig,
    faulted: bool,
}
impl UiBridge {
    pub fn new(config: BridgeHealthConfig) -> Self {
        Self {
            bridge_id: None,
            next_heartbeat: None,
            awaiting: None,
            missed: 0,
            config,
            faulted: false,
        }
    }
    pub fn ready(&mut self, bridge_id: BridgeId, now: Instant) {
        self.bridge_id = Some(bridge_id);
        self.next_heartbeat = Some(now + self.config.interval);
        self.awaiting = None;
        self.missed = 0;
        self.faulted = false;
    }
    pub fn bridge_id(&self) -> Option<&BridgeId> {
        self.bridge_id.as_ref()
    }
    pub fn accepts(&self, bridge_id: &BridgeId) -> bool {
        !self.faulted && self.bridge_id.as_ref() == Some(bridge_id)
    }
    pub fn due(&mut self, now: Instant) -> Option<u64> {
        if self.next_heartbeat.is_none_or(|deadline| now < deadline) || self.faulted {
            return None;
        }
        if self.awaiting.is_some() {
            self.missed = self.missed.saturating_add(1);
        }
        if self.missed >= self.config.missed_ack_threshold {
            self.faulted = true;
            return None;
        }
        let sequence = self.awaiting.unwrap_or(0).saturating_add(1);
        self.awaiting = Some(sequence);
        self.next_heartbeat = Some(now + self.config.interval);
        Some(sequence)
    }
    pub fn acknowledge(&mut self, bridge_id: &BridgeId, sequence: u64) -> bool {
        if !self.accepts(bridge_id) || self.awaiting != Some(sequence) {
            return false;
        }
        self.awaiting = None;
        self.missed = 0;
        true
    }
    pub fn failed(&self) -> bool {
        self.faulted
    }
    pub fn next_deadline(&self) -> Option<Instant> {
        self.next_heartbeat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_missed_acks_fail_the_bridge() {
        let now = Instant::now();
        let mut bridge = UiBridge::new(BridgeHealthConfig {
            interval: Duration::from_millis(1),
            missed_ack_threshold: 3,
        });
        bridge.ready(BridgeId::new("bridge-1".into()).unwrap(), now);
        assert!(bridge.due(now + Duration::from_millis(1)).is_some());
        assert!(bridge.due(now + Duration::from_millis(2)).is_some());
        assert!(bridge.due(now + Duration::from_millis(3)).is_some());
        let _ = bridge.due(now + Duration::from_millis(4));
        assert!(bridge.failed());
    }
}
