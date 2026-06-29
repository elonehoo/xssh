use gpui::SharedString;
use gpui_component::{IndexPath, select::SelectItem};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    Zh,
    En,
    Ja,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum TextKey {
    Vault,
    Hosts,
    SearchHosts,
    CreateHost,
    EditHost,
    Terminal,
    SortNewest,
    EmptyHosts,
    TerminalDisconnected,
    TerminalEmpty,
    MissingTab,
    Name,
    Hostname,
    Port,
    Authentication,
    Username,
    Password,
    Cancel,
    Save,
    Delete,
    DeleteHost,
    SelectAuthentication,
    DialogReady,
    EditDialogReady,
    ManualPassword,
    DirectKey,
    Settings,
    Theme,
    LightTheme,
    DarkTheme,
    Language,
}

impl Language {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Zh => "中文",
            Self::En => "English",
            Self::Ja => "日本語",
        }
    }

    pub(crate) fn selected_index(self) -> IndexPath {
        let row = match self {
            Self::Zh => 0,
            Self::En => 1,
            Self::Ja => 2,
        };
        IndexPath::default().row(row)
    }

    pub(crate) fn tr(self, key: TextKey) -> &'static str {
        match self {
            Self::Zh => match key {
                TextKey::Vault => "Vault",
                TextKey::Hosts => "Hosts",
                TextKey::SearchHosts => "搜索 Hosts...",
                TextKey::CreateHost => "创建 Host",
                TextKey::EditHost => "修改 Host",
                TextKey::Terminal => "终端",
                TextKey::SortNewest => "从新到旧",
                TextKey::EmptyHosts => "还没有服务器，先创建一台 Host。",
                TextKey::TerminalDisconnected => "已断开",
                TextKey::TerminalEmpty => "等待终端输出。",
                TextKey::MissingTab => "这个服务器 tab 已不可用。",
                TextKey::Name => "名称",
                TextKey::Hostname => "Hostname",
                TextKey::Port => "端口",
                TextKey::Authentication => "认证方式",
                TextKey::Username => "用户名",
                TextKey::Password => "密码",
                TextKey::Cancel => "取消",
                TextKey::Save => "保存",
                TextKey::Delete => "删除",
                TextKey::DeleteHost => "删除 Host",
                TextKey::SelectAuthentication => "选择认证方式",
                TextKey::DialogReady => "填写 Host 信息后保存。",
                TextKey::EditDialogReady => "修改 Host 信息后保存。",
                TextKey::ManualPassword => "手动密码",
                TextKey::DirectKey => "直接密钥",
                TextKey::Settings => "设置",
                TextKey::Theme => "主题",
                TextKey::LightTheme => "浅色",
                TextKey::DarkTheme => "深色",
                TextKey::Language => "语言",
            },
            Self::En => match key {
                TextKey::Vault => "Vault",
                TextKey::Hosts => "Hosts",
                TextKey::SearchHosts => "Search hosts...",
                TextKey::CreateHost => "Create host",
                TextKey::EditHost => "Edit host",
                TextKey::Terminal => "Terminal",
                TextKey::SortNewest => "Newest to oldest",
                TextKey::EmptyHosts => "No servers yet. Create a host first.",
                TextKey::TerminalDisconnected => "Disconnected",
                TextKey::TerminalEmpty => "Waiting for terminal output.",
                TextKey::MissingTab => "This server tab is no longer available.",
                TextKey::Name => "Name",
                TextKey::Hostname => "Hostname",
                TextKey::Port => "Port",
                TextKey::Authentication => "Authentication",
                TextKey::Username => "Username",
                TextKey::Password => "Password",
                TextKey::Cancel => "Cancel",
                TextKey::Save => "Save",
                TextKey::Delete => "Delete",
                TextKey::DeleteHost => "Delete host",
                TextKey::SelectAuthentication => "Select authentication",
                TextKey::DialogReady => "Fill in the host details, then save.",
                TextKey::EditDialogReady => "Update the host details, then save.",
                TextKey::ManualPassword => "Manual Password",
                TextKey::DirectKey => "Direct key",
                TextKey::Settings => "Settings",
                TextKey::Theme => "Theme",
                TextKey::LightTheme => "Light",
                TextKey::DarkTheme => "Dark",
                TextKey::Language => "Language",
            },
            Self::Ja => match key {
                TextKey::Vault => "Vault",
                TextKey::Hosts => "Hosts",
                TextKey::SearchHosts => "Hosts を検索...",
                TextKey::CreateHost => "Host を作成",
                TextKey::EditHost => "Host を編集",
                TextKey::Terminal => "ターミナル",
                TextKey::SortNewest => "新しい順",
                TextKey::EmptyHosts => "サーバーがありません。まず Host を作成してください。",
                TextKey::TerminalDisconnected => "切断済み",
                TextKey::TerminalEmpty => "ターミナル出力を待機中。",
                TextKey::MissingTab => "このサーバー tab は利用できません。",
                TextKey::Name => "名前",
                TextKey::Hostname => "Hostname",
                TextKey::Port => "ポート",
                TextKey::Authentication => "認証方式",
                TextKey::Username => "ユーザー名",
                TextKey::Password => "パスワード",
                TextKey::Cancel => "キャンセル",
                TextKey::Save => "保存",
                TextKey::Delete => "削除",
                TextKey::DeleteHost => "Host を削除",
                TextKey::SelectAuthentication => "認証方式を選択",
                TextKey::DialogReady => "Host 情報を入力して保存してください。",
                TextKey::EditDialogReady => "Host 情報を変更して保存してください。",
                TextKey::ManualPassword => "手動パスワード",
                TextKey::DirectKey => "直接キー",
                TextKey::Settings => "設定",
                TextKey::Theme => "テーマ",
                TextKey::LightTheme => "ライト",
                TextKey::DarkTheme => "ダーク",
                TextKey::Language => "言語",
            },
        }
    }

    pub(crate) fn auth_options(self) -> Vec<&'static str> {
        vec![
            self.tr(TextKey::ManualPassword),
            self.tr(TextKey::DirectKey),
        ]
    }

    pub(crate) fn delete_host_message(self, name: &str) -> String {
        match self {
            Self::Zh => format!("确定要删除「{name}」吗？这个操作无法撤销。"),
            Self::En => format!("Delete \"{name}\"? This action cannot be undone."),
            Self::Ja => format!("「{name}」を削除しますか？この操作は元に戻せません。"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct LanguageChoice {
    language: Language,
    title: SharedString,
}

impl LanguageChoice {
    pub(crate) fn new(language: Language) -> Self {
        Self {
            language,
            title: language.label().into(),
        }
    }
}

impl SelectItem for LanguageChoice {
    type Value = Language;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.language
    }
}
