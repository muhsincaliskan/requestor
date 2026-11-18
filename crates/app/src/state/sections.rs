#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorSection {
    #[default]
    Params,
    Headers,
    Body,
    Auth,
    Tests,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseSection {
    #[default]
    Body,
    Headers,
    Cookies,
    Tests,
    Changes,
}

impl EditorSection {
    pub const ALL: [EditorSection; 5] = [
        EditorSection::Params,
        EditorSection::Headers,
        EditorSection::Body,
        EditorSection::Auth,
        EditorSection::Tests,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EditorSection::Params => "Params",
            EditorSection::Headers => "Headers",
            EditorSection::Body => "Body",
            EditorSection::Auth => "Auth",
            EditorSection::Tests => "Tests",
        }
    }
}

impl ResponseSection {
    pub const ALL: [ResponseSection; 5] = [
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
        }
    }
}
