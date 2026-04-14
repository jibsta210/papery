pub mod appearance;
pub mod history;
pub mod schedule;
pub mod sources;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Sources,
    Schedule,
    Appearance,
    History,
}

impl Page {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Sources => "Sources",
            Self::Schedule => "Schedule",
            Self::Appearance => "Appearance",
            Self::History => "History",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Sources => "applications-internet-symbolic",
            Self::Schedule => "preferences-system-time-symbolic",
            Self::Appearance => "preferences-desktop-wallpaper-symbolic",
            Self::History => "document-open-recent-symbolic",
        }
    }
}
