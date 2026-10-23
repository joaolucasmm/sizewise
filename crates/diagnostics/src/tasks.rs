//! Scheduled tasks that run at logon, read through the Task Scheduler.
//!
//! Windows' own tasks (the `\Microsoft` folder) are left out: they are part
//! of Windows, not programs the user installed.

use std::path::Path;

use windows::core::{Interface, BSTR};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::TaskScheduler::{
    IExecAction, ITaskFolder, ITaskService, TaskScheduler, TASK_ACTION_EXEC, TASK_ACTION_TYPE,
    TASK_ENUM_HIDDEN, TASK_TRIGGER_LOGON, TASK_TRIGGER_TYPE2,
};
use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};

use crate::startup::{StartupItem, StartupSource, StartupState};

pub fn logon_tasks() -> Vec<StartupItem> {
    crate::com::on_com_thread(|| read_tasks().unwrap_or_default())
}

fn read_tasks() -> windows::core::Result<Vec<StartupItem>> {
    // SAFETY: COM is initialized on this thread by `on_com_thread`.
    unsafe {
        let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)?;
        let empty = VARIANT::default();
        service.Connect(&empty, &empty, &empty, &empty)?;
        let root = service.GetFolder(&BSTR::from(r"\"))?;
        let mut items = Vec::new();
        walk(&root, &mut items);
        Ok(items)
    }
}

/// A VARIANT holding a 32-bit integer, as collection indexes require.
fn index(i: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_I4,
                Anonymous: VARIANT_0_0_0 { lVal: i },
                ..Default::default()
            }),
        },
    }
}

/// Collects logon tasks of `folder` and its subfolders. Folders the user
/// can't read are skipped.
unsafe fn walk(folder: &ITaskFolder, items: &mut Vec<StartupItem>) {
    if let Ok(tasks) = folder.GetTasks(TASK_ENUM_HIDDEN.0) {
        for i in 1..=tasks.Count().unwrap_or(0) {
            if let Ok(task) = tasks.get_Item(&index(i)) {
                if let Some(item) = logon_item(&task) {
                    items.push(item);
                }
            }
        }
    }
    if let Ok(folders) = folder.GetFolders(0) {
        for i in 1..=folders.Count().unwrap_or(0) {
            let Ok(sub) = folders.get_Item(&index(i)) else {
                continue;
            };
            let is_windows = sub
                .Path()
                .is_ok_and(|p| p.to_string().eq_ignore_ascii_case(r"\Microsoft"));
            if !is_windows {
                walk(&sub, items);
            }
        }
    }
}

unsafe fn logon_item(
    task: &windows::Win32::System::TaskScheduler::IRegisteredTask,
) -> Option<StartupItem> {
    let definition = task.Definition().ok()?;

    let triggers = definition.Triggers().ok()?;
    let mut count = 0;
    triggers.Count(&mut count).ok()?;
    let at_logon = (1..=count).any(|i| {
        triggers.get_Item(i).is_ok_and(|trigger| {
            let mut kind = TASK_TRIGGER_TYPE2::default();
            trigger.Type(&mut kind).is_ok() && kind == TASK_TRIGGER_LOGON
        })
    });
    if !at_logon {
        return None;
    }

    // The first program the task runs (tasks may also send e-mails or show
    // messages, which have no program).
    let actions = definition.Actions().ok()?;
    let mut count = 0;
    actions.Count(&mut count).ok()?;
    let (program, arguments) = (1..=count).find_map(|i| {
        let action = actions.get_Item(i).ok()?;
        let mut kind = TASK_ACTION_TYPE::default();
        action.Type(&mut kind).ok()?;
        if kind != TASK_ACTION_EXEC {
            return None;
        }
        let exec: IExecAction = action.cast().ok()?;
        let (mut path, mut arguments) = (BSTR::new(), BSTR::new());
        exec.Path(&mut path).ok()?;
        let _ = exec.Arguments(&mut arguments);
        // Some tasks store the path already quoted.
        let path = path.to_string().trim().trim_matches('"').to_string();
        Some((path, arguments.to_string()))
    })?;

    let command = if arguments.is_empty() {
        program.clone()
    } else {
        format!("\"{program}\" {arguments}")
    };
    let target = crate::command::parse(&format!("\"{program}\"")).map(|c| c.program);
    Some(StartupItem {
        name: task.Name().ok()?.to_string(),
        publisher: None,
        command,
        target_exists: Some(target.as_deref().is_some_and(Path::is_file)),
        target,
        source: StartupSource::ScheduledTask {
            path: task.Path().ok()?.to_string(),
        },
        state: if task.Enabled().ok()?.as_bool() {
            StartupState::Enabled
        } else {
            StartupState::Disabled
        },
    })
}
