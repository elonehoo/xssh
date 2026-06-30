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
    ConnectHost,
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
    TestConnection,
    Delete,
    DeleteHost,
    SelectAuthentication,
    ManualPassword,
    DirectKey,
    ExpandSidebar,
    CollapseSidebar,
    Settings,
    Appearance,
    Theme,
    TerminalTheme,
    DarkTerminalTheme,
    LightTerminalTheme,
    LightTheme,
    DarkTheme,
    Language,
}

impl Language {
    pub(crate) fn from_setting_value(value: &str) -> Self {
        match value {
            "en" => Self::En,
            "ja" => Self::Ja,
            _ => Self::Zh,
        }
    }

    pub(crate) fn setting_value(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
            Self::Ja => "ja",
        }
    }

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
                TextKey::ConnectHost => "连接 Host",
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
                TextKey::TestConnection => "测试连接",
                TextKey::Delete => "删除",
                TextKey::DeleteHost => "删除 Host",
                TextKey::SelectAuthentication => "选择认证方式",
                TextKey::ManualPassword => "手动密码",
                TextKey::DirectKey => "直接密钥",
                TextKey::ExpandSidebar => "展开侧边栏",
                TextKey::CollapseSidebar => "收起侧边栏",
                TextKey::Settings => "设置",
                TextKey::Appearance => "外观",
                TextKey::Theme => "主题",
                TextKey::TerminalTheme => "终端主题",
                TextKey::DarkTerminalTheme => "暗色终端主题",
                TextKey::LightTerminalTheme => "亮色终端主题",
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
                TextKey::ConnectHost => "Connect host",
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
                TextKey::TestConnection => "Test connection",
                TextKey::Delete => "Delete",
                TextKey::DeleteHost => "Delete host",
                TextKey::SelectAuthentication => "Select authentication",
                TextKey::ManualPassword => "Manual Password",
                TextKey::DirectKey => "Direct key",
                TextKey::ExpandSidebar => "Expand sidebar",
                TextKey::CollapseSidebar => "Collapse sidebar",
                TextKey::Settings => "Settings",
                TextKey::Appearance => "Appearance",
                TextKey::Theme => "Theme",
                TextKey::TerminalTheme => "Terminal theme",
                TextKey::DarkTerminalTheme => "Dark terminal theme",
                TextKey::LightTerminalTheme => "Light terminal theme",
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
                TextKey::ConnectHost => "Host に接続",
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
                TextKey::TestConnection => "接続テスト",
                TextKey::Delete => "削除",
                TextKey::DeleteHost => "Host を削除",
                TextKey::SelectAuthentication => "認証方式を選択",
                TextKey::ManualPassword => "手動パスワード",
                TextKey::DirectKey => "直接キー",
                TextKey::ExpandSidebar => "サイドバーを展開",
                TextKey::CollapseSidebar => "サイドバーを折りたたむ",
                TextKey::Settings => "設定",
                TextKey::Appearance => "外観",
                TextKey::Theme => "テーマ",
                TextKey::TerminalTheme => "ターミナルテーマ",
                TextKey::DarkTerminalTheme => "ダークターミナルテーマ",
                TextKey::LightTerminalTheme => "ライトターミナルテーマ",
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
