use ksni::{self, menu::StandardItem, ToolTip, TrayMethods};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum TrayAction {
    ShowWindow,
    NextWallpaper,
    TogglePause,
    Quit,
}

// Global channel for tray → app communication
static TRAY_TX: OnceLock<mpsc::UnboundedSender<TrayAction>> = OnceLock::new();

fn send_action(action: TrayAction) {
    if let Some(tx) = TRAY_TX.get() {
        let _ = tx.send(action);
    }
}

pub fn take_receiver() -> mpsc::UnboundedReceiver<TrayAction> {
    let (tx, rx) = mpsc::unbounded_channel();
    let _ = TRAY_TX.set(tx);
    rx
}

struct PaperyTray {
    paused: Arc<AtomicBool>,
}

impl ksni::Tray for PaperyTray {
    fn id(&self) -> String {
        "dev-papery".to_string()
    }

    fn title(&self) -> String {
        "Papery".to_string()
    }

    fn icon_name(&self) -> String {
        "preferences-desktop-wallpaper".to_string()
    }

    fn tool_tip(&self) -> ToolTip {
        let status = if self.paused.load(Ordering::Relaxed) {
            "Paused"
        } else {
            "Running"
        };
        ToolTip {
            title: format!("Papery - {status}"),
            description: String::new(),
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let paused = self.paused.load(Ordering::Relaxed);
        vec![
            StandardItem {
                label: "Show Papery".to_string(),
                activate: Box::new(|_: &mut Self| send_action(TrayAction::ShowWindow)),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Next Wallpaper".to_string(),
                activate: Box::new(|_: &mut Self| send_action(TrayAction::NextWallpaper)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if paused {
                    "Resume".to_string()
                } else {
                    "Pause".to_string()
                },
                activate: Box::new(|_: &mut Self| send_action(TrayAction::TogglePause)),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(|_: &mut Self| send_action(TrayAction::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        send_action(TrayAction::ShowWindow);
    }
}

pub fn spawn_tray(paused: Arc<AtomicBool>) {
    tokio::spawn(async move {
        // Retry loop — StatusNotifierWatcher may not be ready at boot
        for attempt in 1..=30 {
            let tray = PaperyTray {
                paused: paused.clone(),
            };
            match tray.spawn().await {
                Ok(_handle) => {
                    tracing::info!("System tray icon registered (attempt {attempt})");
                    std::future::pending::<()>().await;
                    return;
                }
                Err(e) => {
                    tracing::warn!("Tray attempt {attempt}/30 failed: {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
        tracing::error!("Failed to start tray after 30 attempts");
    });
}
