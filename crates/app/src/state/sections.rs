#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorSection {
    #[default]
    Params,
    Headers,
    Body,
    Auth,
    Tests,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseSection {
    #[default]
    Body,
    Headers,
    Cookies,
    Tests,
    Changes,
    Events,
}

impl EditorSection {
    pub const ALL: [EditorSection; 6] = [
        EditorSection::Params,
        EditorSection::Headers,
        EditorSection::Body,
        EditorSection::Auth,
        EditorSection::Tests,
        EditorSection::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EditorSection::Params => "Params",
            EditorSection::Headers => "Headers",
            EditorSection::Body => "Body",
            EditorSection::Auth => "Auth",
            EditorSection::Tests => "Tests",
            EditorSection::Settings => "Settings",
        }
    }
}

impl ResponseSection {
    pub const ALL: [ResponseSection; 6] = [
        ResponseSection::Events,
        ResponseSection::Body,
        ResponseSection::Headers,
        ResponseSection::Cookies,
        ResponseSection::Tests,
        ResponseSection::Changes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ResponseSection::Body => "Body",
            ResponseSection::Headers => "Headers",
            ResponseSection::Cookies => "Cookies",
            ResponseSection::Tests => "Tests",
            ResponseSection::Changes => "Changes",
            ResponseSection::Events => "Events",
        }
    }
}
