use std::collections::HashMap;

use crate::board::BoardSignature;
use crate::stats::AdvanceOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternBackend {
    InMemory,
    Streaming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    Cycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternSpan {
    pub start_generation: u64,
    pub end_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CyclePattern {
    pub start_generation: u64,
    pub detected_generation: u64,
    pub period: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternMatchDetails {
    Cycle(CyclePattern),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternMatch {
    pub kind: PatternKind,
    pub detected_generation: u64,
    pub span: Option<PatternSpan>,
    pub terminal: bool,
    pub details: PatternMatchDetails,
}

impl PatternMatch {
    pub fn cycle(cycle: CyclePattern) -> Self {
        Self {
            kind: PatternKind::Cycle,
            detected_generation: cycle.detected_generation,
            span: Some(PatternSpan {
                start_generation: cycle.start_generation,
                end_generation: cycle.detected_generation,
            }),
            terminal: true,
            details: PatternMatchDetails::Cycle(cycle),
        }
    }

    pub fn cycle_pattern(self) -> CyclePattern {
        match self.details {
            PatternMatchDetails::Cycle(cycle) => cycle,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PatternObservation<'a> {
    pub generation: u64,
    pub backend: PatternBackend,
    pub outcome: Option<AdvanceOutcome>,
    pub signature: Option<&'a BoardSignature>,
}

impl<'a> PatternObservation<'a> {
    pub fn new(
        generation: u64,
        backend: PatternBackend,
        outcome: Option<AdvanceOutcome>,
        signature: Option<&'a BoardSignature>,
    ) -> Self {
        Self {
            generation,
            backend,
            outcome,
            signature,
        }
    }
}

pub trait PatternDetector: Send {
    fn observe(&mut self, observation: &PatternObservation<'_>) -> Option<PatternMatch>;
}

#[derive(Default)]
pub struct PatternAnalyzer {
    detectors: Vec<Box<dyn PatternDetector>>,
}

impl PatternAnalyzer {
    pub fn in_memory_cycle_detection() -> Self {
        Self {
            detectors: vec![Box::new(CycleDetector::default())],
        }
    }

    pub fn observe(&mut self, observation: &PatternObservation<'_>) -> Option<PatternMatch> {
        for detector in &mut self.detectors {
            if let Some(pattern_match) = detector.observe(observation) {
                if pattern_match.terminal {
                    return Some(pattern_match);
                }
            }
        }
        None
    }
}

#[derive(Debug, Default)]
pub struct CycleDetector {
    seen: HashMap<BoardSignature, u64>,
}

impl PatternDetector for CycleDetector {
    fn observe(&mut self, observation: &PatternObservation<'_>) -> Option<PatternMatch> {
        if !matches!(observation.backend, PatternBackend::InMemory) {
            return None;
        }
        let signature = observation.signature?;
        if let Some(first_seen_generation) = self.seen.get(signature).copied() {
            let cycle = CyclePattern {
                start_generation: first_seen_generation,
                detected_generation: observation.generation,
                period: observation.generation - first_seen_generation,
            };
            Some(PatternMatch::cycle(cycle))
        } else {
            self.seen.insert(signature.clone(), observation.generation);
            None
        }
    }
}
