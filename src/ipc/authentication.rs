use gpui_component::IndexPath;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthenticationMode {
    ManualPassword,
    DirectKey,
}

impl AuthenticationMode {
    pub(crate) fn storage_label(self) -> &'static str {
        match self {
            Self::ManualPassword => "Manual Password",
            Self::DirectKey => "Direct key",
        }
    }

    pub(crate) fn from_label(label: &str) -> Self {
        match label {
            "Direct key" | "直接密钥" | "直接キー" => Self::DirectKey,
            _ => Self::ManualPassword,
        }
    }

    pub(crate) fn selected_index(self) -> IndexPath {
        let row = match self {
            Self::ManualPassword => 0,
            Self::DirectKey => 1,
        };
        IndexPath::default().row(row)
    }
}
