use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{
    AnyElement, Context, Entity, IntoElement, SharedString, deferred, div, prelude::*, px, rgb,
};
use gpui_component::scroll::ScrollableElement;

use crate::{
    ipc::{UploadTaskEvent, UploadTaskEventKind},
    ui::{AppThemeId, Language, icons},
};

use super::Xssh;

const MAX_UPLOAD_TASKS: usize = 24;
const MAX_UPLOAD_TASK_MESSAGES: usize = 8;
const VISIBLE_UPLOAD_LOG_TASKS: usize = 12;
static NEXT_LOCAL_UPLOAD_TASK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub(super) struct UploadTask {
    id: u64,
    server_name: String,
    remote_directory: Option<String>,
    file_count: usize,
    succeeded: usize,
    failed: usize,
    status: UploadTaskStatus,
    messages: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UploadTaskStatus {
    Running,
    Completed,
    CompletedWithErrors,
    Failed,
}

impl UploadTask {
    fn started(
        task_id: u64,
        server_name: String,
        remote_directory: Option<String>,
        file_count: usize,
    ) -> Self {
        Self {
            id: task_id,
            server_name,
            remote_directory,
            file_count,
            succeeded: 0,
            failed: 0,
            status: UploadTaskStatus::Running,
            messages: Vec::new(),
        }
    }

    fn display_done(&self) -> usize {
        self.succeeded + self.failed
    }

    fn latest_message(&self) -> Option<&str> {
        self.messages.last().map(String::as_str)
    }

    fn previous_messages(&self) -> impl Iterator<Item = &str> {
        self.messages
            .iter()
            .rev()
            .skip(1)
            .take(2)
            .map(String::as_str)
    }

    fn push_message(&mut self, message: impl Into<String>) {
        let message = message.into();
        if message.is_empty() {
            return;
        }

        self.messages.push(message);
        if self.messages.len() > MAX_UPLOAD_TASK_MESSAGES {
            self.messages.remove(0);
        }
    }

    fn record_file_succeeded(&mut self, local_path: String, remote_path: String) {
        self.succeeded += 1;
        self.push_message(format!(
            "{} -> {}",
            file_name_display(&local_path),
            file_name_display(&remote_path)
        ));
    }

    fn record_file_failed(&mut self, local_path: String, error: String) {
        self.failed += 1;
        self.push_message(format!("{}: {error}", file_name_display(&local_path)));
    }

    fn finish(&mut self, language: Language, succeeded: usize, failed: usize) {
        self.succeeded = succeeded;
        self.failed = failed;
        self.status = if failed == 0 {
            UploadTaskStatus::Completed
        } else {
            UploadTaskStatus::CompletedWithErrors
        };
        self.push_message(upload_task_finished_message(language, succeeded, failed));
    }

    fn fail(&mut self, error: String) {
        self.status = UploadTaskStatus::Failed;
        self.push_message(error);
    }
}

impl Xssh {
    pub(in crate::pages::index) fn add_failed_upload_task(
        &mut self,
        server_name: String,
        file_count: usize,
        error: impl Into<String>,
    ) {
        let task_id = next_local_upload_task_id();
        self.apply_upload_task_event(UploadTaskEvent {
            task_id,
            server_name: server_name.clone(),
            kind: UploadTaskEventKind::Started {
                remote_directory: None,
                file_count,
            },
        });
        self.apply_upload_task_event(UploadTaskEvent {
            task_id,
            server_name,
            kind: UploadTaskEventKind::Failed {
                error: error.into(),
            },
        });
    }

    pub(in crate::pages::index) fn upload_log_menu(&self, view: Entity<Self>) -> impl IntoElement {
        let button_view = view.clone();
        let panel_view = view.clone();
        let close_view = view;

        div()
            .relative()
            .flex()
            .items_center()
            .child(self.upload_log_button(button_view))
            .when(self.upload_log_open, |this| {
                this.child(self.upload_log_panel(panel_view))
            })
            .when(self.upload_log_open, |this| {
                this.on_mouse_down_out(move |_, _, cx| {
                    close_view.update(cx, |this, cx| {
                        this.close_upload_log(cx);
                    });
                })
            })
    }

    pub(in crate::pages::index) fn apply_upload_task_event(&mut self, event: UploadTaskEvent) {
        let language = self.language;

        match event.kind {
            UploadTaskEventKind::Started {
                remote_directory,
                file_count,
            } => {
                self.upload_tasks.retain(|task| task.id != event.task_id);
                let mut task = UploadTask::started(
                    event.task_id,
                    event.server_name,
                    remote_directory,
                    file_count,
                );
                task.push_message(upload_task_fallback_detail(language, &task));
                self.upload_tasks.insert(0, task);
                self.upload_tasks.truncate(MAX_UPLOAD_TASKS);
            }
            UploadTaskEventKind::FileSucceeded {
                local_path,
                remote_path,
            } => {
                if let Some(task) = self.upload_task_mut(event.task_id) {
                    task.record_file_succeeded(local_path, remote_path);
                }
            }
            UploadTaskEventKind::FileFailed { local_path, error } => {
                if let Some(task) = self.upload_task_mut(event.task_id) {
                    task.record_file_failed(local_path, error);
                }
            }
            UploadTaskEventKind::Finished { succeeded, failed } => {
                if let Some(task) = self.upload_task_mut(event.task_id) {
                    task.finish(language, succeeded, failed);
                }
            }
            UploadTaskEventKind::Failed { error } => {
                if let Some(task) = self.upload_task_mut(event.task_id) {
                    task.fail(error);
                }
            }
        }
    }

    pub(in crate::pages::index) fn upload_log_button(
        &self,
        view: Entity<Self>,
    ) -> impl IntoElement {
        let palette = self.theme.palette();
        let group_name = SharedString::from("upload-log-button");
        let tooltip = upload_log_title(self.language);
        let foreground = if self.upload_log_open {
            palette.text
        } else {
            palette.muted
        };
        let indicator_color = self.upload_log_indicator_color();

        div()
            .id("upload-log-button")
            .group(group_name.clone())
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .size(px(28.))
            .rounded_sm()
            .border_1()
            .border_color(if self.upload_log_open {
                rgb(palette.card_active_border)
            } else {
                rgb(palette.border)
            })
            .bg(if self.upload_log_open {
                rgb(palette.tab_active)
            } else {
                rgb(palette.tab_inactive)
            })
            .hover(move |style| {
                style
                    .bg(rgb(palette.button_hover))
                    .border_color(rgb(palette.card_active_border))
            })
            .child(icons::render(
                icons::notification_status::INFO_PATH,
                15.,
                foreground,
            ))
            .when_some(indicator_color, |this, color| {
                this.child(
                    div()
                        .absolute()
                        .top(px(4.))
                        .right(px(4.))
                        .size(px(6.))
                        .rounded_sm()
                        .bg(rgb(color)),
                )
            })
            .child(upload_log_tooltip(
                self.theme,
                group_name,
                SharedString::from(tooltip),
            ))
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                view.update(cx, |this, cx| {
                    this.upload_log_open = !this.upload_log_open;
                    cx.notify();
                });
            })
    }

    pub(in crate::pages::index) fn upload_log_panel(&self, view: Entity<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let rows = if self.upload_tasks.is_empty() {
            vec![self.upload_log_empty_row().into_any_element()]
        } else {
            self.upload_tasks
                .iter()
                .take(VISIBLE_UPLOAD_LOG_TASKS)
                .map(|task| self.upload_log_row(task, view.clone()).into_any_element())
                .collect::<Vec<_>>()
        };
        let close_view = view.clone();

        deferred(
            div()
                .id("upload-log-panel")
                .absolute()
                .top(px(34.))
                .right(px(0.))
                .w(px(390.))
                .max_h(px(420.))
                .overflow_hidden()
                .rounded_md()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.panel_bg))
                .shadow_md()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .h(px(38.))
                        .px_3()
                        .border_b_1()
                        .border_color(rgb(palette.separator))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_size(px(13.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(rgb(palette.text))
                                .child(icons::render(
                                    icons::notification_status::INFO_PATH,
                                    15.,
                                    palette.text,
                                ))
                                .child(upload_log_title(self.language)),
                        )
                        .child(
                            div()
                                .id("upload-log-close")
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(24.))
                                .rounded_sm()
                                .child(icons::close::icon(11., palette.muted))
                                .hover(move |style| style.bg(rgb(palette.button_hover)))
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    close_view.update(cx, |this, cx| {
                                        this.close_upload_log(cx);
                                    });
                                }),
                        ),
                )
                .child(div().max_h(px(360.)).overflow_y_scrollbar().children(rows)),
        )
        .with_priority(3)
    }

    fn upload_log_row(&self, task: &UploadTask, view: Entity<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let status_color = self.upload_task_status_color(task.status);
        let progress = upload_task_progress_label(self.language, task);
        let detail = upload_task_detail_label(self.language, task);
        let task_id = task.id;
        let messages = task
            .previous_messages()
            .map(|message| {
                div()
                    .min_w(px(0.))
                    .truncate()
                    .text_color(rgb(palette.muted))
                    .child(message.to_string())
                    .into_any_element()
            })
            .collect::<Vec<AnyElement>>();

        div()
            .id(("upload-log-task", task.id))
            .flex()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(palette.separator))
            .text_size(px(12.))
            .text_color(rgb(palette.text))
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_none()
                    .h(px(18.))
                    .child(div().size(px(7.)).rounded_sm().bg(rgb(status_color))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_1()
                    .min_w(px(0.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .truncate()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(progress),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .max_w(px(150.))
                                    .truncate()
                                    .text_color(rgb(palette.label))
                                    .child(task.server_name.clone()),
                            ),
                    )
                    .child(
                        div()
                            .min_w(px(0.))
                            .truncate()
                            .text_color(rgb(palette.muted))
                            .child(detail),
                    )
                    .children(messages),
            )
            .child(
                div()
                    .id(("upload-log-delete", task_id))
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_none()
                    .size(px(24.))
                    .rounded_sm()
                    .child(icons::render(icons::delete::PATH, 13., palette.muted))
                    .hover(move |style| style.bg(rgb(palette.button_hover)))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        view.update(cx, |this, cx| {
                            this.remove_upload_log_task(task_id, cx);
                        });
                    }),
            )
    }

    fn upload_log_empty_row(&self) -> impl IntoElement {
        let palette = self.theme.palette();

        div()
            .h(px(72.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.))
            .text_color(rgb(palette.muted))
            .child(upload_log_empty_text(self.language))
    }

    fn upload_task_status_color(&self, status: UploadTaskStatus) -> u32 {
        let terminal_palette = self.active_terminal_palette();

        match status {
            UploadTaskStatus::Running => terminal_palette.ansi[4],
            UploadTaskStatus::Completed => terminal_palette.ansi[2],
            UploadTaskStatus::CompletedWithErrors => terminal_palette.ansi[3],
            UploadTaskStatus::Failed => terminal_palette.ansi[1],
        }
    }

    fn upload_task_mut(&mut self, task_id: u64) -> Option<&mut UploadTask> {
        self.upload_tasks.iter_mut().find(|task| task.id == task_id)
    }

    fn remove_upload_log_task(&mut self, task_id: u64, cx: &mut Context<Self>) {
        remove_upload_task_by_id(&mut self.upload_tasks, task_id);
        cx.notify();
    }

    fn close_upload_log(&mut self, cx: &mut Context<Self>) {
        if self.upload_log_open {
            self.upload_log_open = false;
            cx.notify();
        }
    }

    fn upload_log_indicator_color(&self) -> Option<u32> {
        let first_matching = |status| {
            self.upload_tasks
                .iter()
                .any(|task| task.status == status)
                .then(|| self.upload_task_status_color(status))
        };

        first_matching(UploadTaskStatus::Failed)
            .or_else(|| first_matching(UploadTaskStatus::CompletedWithErrors))
            .or_else(|| first_matching(UploadTaskStatus::Running))
            .or_else(|| first_matching(UploadTaskStatus::Completed))
    }
}

fn upload_task_progress_label(language: Language, task: &UploadTask) -> String {
    match (language, task.status) {
        (Language::Zh, UploadTaskStatus::Running) => {
            format!("上传 {}/{}", task.display_done(), task.file_count)
        }
        (Language::Zh, UploadTaskStatus::Completed) => "上传完成".to_string(),
        (Language::Zh, UploadTaskStatus::CompletedWithErrors) => "部分失败".to_string(),
        (Language::Zh, UploadTaskStatus::Failed) => "上传失败".to_string(),
        (Language::En, UploadTaskStatus::Running) => {
            format!("Upload {}/{}", task.display_done(), task.file_count)
        }
        (Language::En, UploadTaskStatus::Completed) => "Uploaded".to_string(),
        (Language::En, UploadTaskStatus::CompletedWithErrors) => "Partial".to_string(),
        (Language::En, UploadTaskStatus::Failed) => "Failed".to_string(),
        (Language::Ja, UploadTaskStatus::Running) => {
            format!("アップロード {}/{}", task.display_done(), task.file_count)
        }
        (Language::Ja, UploadTaskStatus::Completed) => "完了".to_string(),
        (Language::Ja, UploadTaskStatus::CompletedWithErrors) => "一部失敗".to_string(),
        (Language::Ja, UploadTaskStatus::Failed) => "失敗".to_string(),
    }
}

fn upload_task_detail_label(language: Language, task: &UploadTask) -> String {
    if let Some(message) = task.latest_message() {
        return message.to_string();
    }

    upload_task_fallback_detail(language, task)
}

fn upload_task_fallback_detail(language: Language, task: &UploadTask) -> String {
    if let Some(remote_directory) = &task.remote_directory {
        return format!("{} -> {remote_directory}", task.server_name);
    }

    match language {
        Language::Zh => format!("{} 等待目录", task.server_name),
        Language::En => format!("{} waiting for path", task.server_name),
        Language::Ja => format!("{} パス待ち", task.server_name),
    }
}

fn upload_task_finished_message(language: Language, succeeded: usize, failed: usize) -> String {
    match language {
        Language::Zh if failed == 0 => format!("全部完成，成功 {succeeded} 个"),
        Language::Zh => format!("完成，成功 {succeeded} 个，失败 {failed} 个"),
        Language::En if failed == 0 => format!("Done, {succeeded} succeeded"),
        Language::En => format!("Done, {succeeded} succeeded, {failed} failed"),
        Language::Ja if failed == 0 => format!("完了、成功 {succeeded} 件"),
        Language::Ja => format!("完了、成功 {succeeded} 件、失敗 {failed} 件"),
    }
}

fn upload_log_title(language: Language) -> &'static str {
    match language {
        Language::Zh => "上传日志",
        Language::En => "Upload Log",
        Language::Ja => "アップロードログ",
    }
}

fn upload_log_empty_text(language: Language) -> &'static str {
    match language {
        Language::Zh => "暂无上传记录",
        Language::En => "No uploads yet",
        Language::Ja => "アップロード履歴はありません",
    }
}

fn upload_log_tooltip(
    theme: AppThemeId,
    group_name: SharedString,
    tooltip: SharedString,
) -> impl IntoElement {
    let palette = theme.palette();
    let tooltip_group = group_name.clone();

    deferred(
        div()
            .group(tooltip_group)
            .absolute()
            .top(px(32.))
            .right(px(0.))
            .flex()
            .h(px(28.))
            .items_center()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.panel_bg))
            .px_2()
            .text_size(px(12.))
            .text_color(rgb(palette.text))
            .whitespace_nowrap()
            .shadow_md()
            .invisible()
            .group_hover(group_name, |style| style.visible())
            .child(tooltip),
    )
    .with_priority(4)
}

fn remove_upload_task_by_id(tasks: &mut Vec<UploadTask>, task_id: u64) {
    tasks.retain(|task| task.id != task_id);
}

fn file_name_display(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| path.to_string())
}

fn next_local_upload_task_id() -> u64 {
    u64::MAX - NEXT_LOCAL_UPLOAD_TASK_ID.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_task_messages_keep_recent_entries() {
        let mut task = UploadTask::started(1, "Server".to_string(), Some("/app".to_string()), 1);

        for index in 0..10 {
            task.push_message(format!("message {index}"));
        }

        assert_eq!(task.messages.len(), MAX_UPLOAD_TASK_MESSAGES);
        assert_eq!(task.messages.first(), Some(&"message 2".to_string()));
        assert_eq!(task.messages.last(), Some(&"message 9".to_string()));
    }

    #[test]
    fn previous_messages_skip_latest_message() {
        let mut task = UploadTask::started(1, "Server".to_string(), Some("/app".to_string()), 1);

        task.push_message("first");
        task.push_message("second");
        task.push_message("latest");

        assert_eq!(task.latest_message(), Some("latest"));
        assert_eq!(
            task.previous_messages().collect::<Vec<_>>(),
            vec!["second", "first"]
        );
    }

    #[test]
    fn formats_finished_upload_messages() {
        assert_eq!(
            upload_task_finished_message(Language::Zh, 2, 0),
            "全部完成，成功 2 个"
        );
        assert_eq!(
            upload_task_finished_message(Language::Zh, 2, 1),
            "完成，成功 2 个，失败 1 个"
        );
    }

    #[test]
    fn removes_only_selected_upload_task() {
        let mut tasks = vec![
            UploadTask::started(1, "A".to_string(), Some("/a".to_string()), 1),
            UploadTask::started(2, "B".to_string(), Some("/b".to_string()), 1),
            UploadTask::started(3, "C".to_string(), Some("/c".to_string()), 1),
        ];

        remove_upload_task_by_id(&mut tasks, 2);

        assert_eq!(
            tasks.iter().map(|task| task.id).collect::<Vec<_>>(),
            vec![1, 3]
        );
    }
}
