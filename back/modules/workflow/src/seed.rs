pub const STAGE_KIND_OPEN: &str = "open";
pub const STAGE_KIND_WON: &str = "won";
pub const STAGE_KIND_LOST: &str = "lost";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DefaultStage {
    pub(crate) name: &'static str,
    pub(crate) probability: u8,
    pub(crate) kind: &'static str,
}

pub(crate) const DEFAULT_STAGES: [DefaultStage; 6] = [
    DefaultStage {
        name: "Start",
        probability: 10,
        kind: STAGE_KIND_OPEN,
    },
    DefaultStage {
        name: "Review",
        probability: 25,
        kind: STAGE_KIND_OPEN,
    },
    DefaultStage {
        name: "Preparation",
        probability: 50,
        kind: STAGE_KIND_OPEN,
    },
    DefaultStage {
        name: "In progress",
        probability: 75,
        kind: STAGE_KIND_OPEN,
    },
    DefaultStage {
        name: "Completed",
        probability: 100,
        kind: STAGE_KIND_WON,
    },
    DefaultStage {
        name: "Cancelled",
        probability: 0,
        kind: STAGE_KIND_LOST,
    },
];
