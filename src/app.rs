use crate::background;
use crate::brightness;
use crate::config::{PaperyConfig, APP_ID};
use crate::download::DownloadManager;
use crate::fl;
use crate::pages::{self, Page};
use crate::scheduler::{self, SchedulerEvent};
use crate::tray::{self, TrayAction};
use crate::wallpaper::bing::BingProvider;
use crate::wallpaper::earth_view::EarthViewProvider;
use crate::wallpaper::local::LocalProvider;
use crate::wallpaper::nasa_apod::NasaApodProvider;
use crate::wallpaper::wallhaven::WallhavenProvider;
use crate::wallpaper::{WallpaperInfo, WallpaperProvider};
use cosmic::iced::futures::{self, stream};
use cosmic::iced::{Length, Subscription};
use cosmic::widget::nav_bar;
use cosmic::{app, executor, widget, Core, Element, Task};
use cosmic_config::CosmicConfigEntry;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub struct Papery {
    core: Core,
    nav_model: nav_bar::Model,
    pub config: PaperyConfig,
    config_handler: Option<cosmic_config::Config>,

    // Wallpaper state
    pub current_wallpaper: Option<WallpaperInfo>,
    pub history: Vec<WallpaperInfo>,
    pub history_index: usize,
    wallpaper_queue: VecDeque<WallpaperInfo>,

    // Timer state
    pub seconds_until_next: u64,
    #[allow(dead_code)]
    last_tick: Option<Instant>,

    // Download manager
    download_manager: DownloadManager,

    // Status
    status_message: Option<String>,
    is_fetching: bool,

    // Interval input state
    pub interval_hours: String,
    pub interval_minutes: String,
    pub interval_seconds: String,

    // System tray
    tray_paused: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Wallpaper actions
    NextWallpaper,
    PreviousWallpaper,
    TogglePause,
    SetWallpaperFromHistory(usize),
    ToggleFavorite(usize),

    // Timer
    TimerTick(SchedulerEvent),

    // Source results
    WallpapersFetched(Result<Vec<WallpaperInfo>, String>),
    WallpaperReady(Result<WallpaperInfo, String>),
    WallpaperSet(Result<(), String>),

    // Config changes
    ConfigChanged(PaperyConfig),
    ToggleSourceBing,
    ToggleSourceNasa,
    ToggleSourceWallhaven,
    ToggleSourceEarthView,
    ToggleSourceLocal,
    SetRotationInterval(u64),
    SetIntervalHours(String),
    SetIntervalMinutes(String),
    SetIntervalSeconds(String),
    SetScalingMode(String),
    SetThemeFilter(String),
    SetBrightnessThreshold(f64),

    // Fetch trigger
    FetchMore,

    // Tray actions
    Tray(TrayAction),
}

#[derive(Debug, Clone)]
pub struct PaperyFlags;

impl cosmic::app::CosmicFlags for PaperyFlags {
    type SubCommand = String;
    type Args = Vec<String>;
}

impl cosmic::Application for Papery {
    type Executor = executor::Default;
    type Flags = PaperyFlags;
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: PaperyFlags) -> (Self, app::Task<Self::Message>) {
        let config_handler = cosmic_config::Config::new(APP_ID, PaperyConfig::VERSION)
            .inspect_err(|e| tracing::error!("Failed to create config handler: {e}"))
            .ok();

        let config = config_handler
            .as_ref()
            .and_then(|h| {
                PaperyConfig::get_entry(h)
                    .inspect_err(|(_errs, _config)| {
                        tracing::warn!("Config had errors, using partial: {_errs:?}");
                    })
                    .ok()
            })
            .unwrap_or_default();

        let cache_dir = DownloadManager::default_cache_dir();
        let download_manager = DownloadManager::new(cache_dir);

        let mut nav_model = nav_bar::Model::default();
        nav_model
            .insert()
            .text(fl!("nav-sources"))
            .icon(widget::icon::from_name(Page::Sources.icon()))
            .data(Page::Sources);
        nav_model
            .insert()
            .text(fl!("nav-schedule"))
            .icon(widget::icon::from_name(Page::Schedule.icon()))
            .data(Page::Schedule);
        nav_model
            .insert()
            .text(fl!("nav-appearance"))
            .icon(widget::icon::from_name(Page::Appearance.icon()))
            .data(Page::Appearance);
        nav_model
            .insert()
            .text(fl!("nav-history"))
            .icon(widget::icon::from_name(Page::History.icon()))
            .data(Page::History);
        nav_model.activate_position(0);

        let mut app_inst = Self {
            core,
            nav_model,
            config,
            config_handler,
            current_wallpaper: None,
            history: Vec::new(),
            history_index: 0,
            wallpaper_queue: VecDeque::new(),
            seconds_until_next: 0,
            last_tick: None,
            download_manager,
            status_message: None,
            is_fetching: false,
            interval_hours: String::new(),
            interval_minutes: String::new(),
            interval_seconds: String::new(),
            tray_paused: Arc::new(AtomicBool::new(false)),
        };

        // Init interval text fields from config
        {
            let total = app_inst.config.rotation_interval_secs;
            app_inst.interval_hours = (total / 3600).to_string();
            app_inst.interval_minutes = ((total % 3600) / 60).to_string();
            app_inst.interval_seconds = (total % 60).to_string();
        }

        // Spawn system tray icon
        tray::spawn_tray(app_inst.tray_paused.clone());

        app_inst.seconds_until_next = app_inst.config.rotation_interval_secs;
        app_inst.tray_paused
            .store(app_inst.config.paused, Ordering::Relaxed);
        app_inst.core.set_header_title(fl!("app-title"));

        let fetch_cmd = Task::done(cosmic::Action::App(Message::FetchMore));

        (app_inst, fetch_cmd)
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav_model)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> app::Task<Self::Message> {
        self.nav_model.activate(id);
        Task::none()
    }

    fn update(&mut self, message: Self::Message) -> app::Task<Self::Message> {
        match message {
            Message::NextWallpaper => {
                if self.history_index + 1 < self.history.len() {
                    self.history_index += 1;
                    let wp = self.history[self.history_index].clone();
                    return self.apply_wallpaper(wp);
                }
                return self.set_next_wallpaper();
            }

            Message::PreviousWallpaper => {
                if self.history_index > 0 {
                    self.history_index -= 1;
                    let wp = self.history[self.history_index].clone();
                    return self.apply_wallpaper(wp);
                }
            }

            Message::TogglePause => {
                self.config.paused = !self.config.paused;
                self.tray_paused
                    .store(self.config.paused, Ordering::Relaxed);
                self.save_config();
                if !self.config.paused {
                    self.seconds_until_next = self.config.rotation_interval_secs;
                    self.last_tick = Some(Instant::now());
                }
            }

            Message::SetWallpaperFromHistory(index) => {
                if index < self.history.len() {
                    self.history_index = index;
                    let wp = self.history[index].clone();
                    return self.apply_wallpaper(wp);
                }
            }

            Message::ToggleFavorite(index) => {
                if let Some(wp) = self.history.get(index) {
                    if let Some(ref path) = wp.local_path {
                        let path_str = path.to_string_lossy().to_string();
                        if let Some(pos) =
                            self.config.favorites.iter().position(|f| f == &path_str)
                        {
                            self.config.favorites.remove(pos);
                        } else {
                            self.config.favorites.push(path_str);
                        }
                        self.save_config();
                    }
                }
            }

            Message::TimerTick(_) => {
                if !self.config.paused {
                    if self.seconds_until_next > 0 {
                        self.seconds_until_next -= 1;
                    }
                    if self.seconds_until_next == 0 {
                        self.seconds_until_next = self.config.rotation_interval_secs;
                        return self.set_next_wallpaper();
                    }
                }
            }

            Message::WallpapersFetched(result) => {
                self.is_fetching = false;
                match result {
                    Ok(wallpapers) => {
                        for wp in wallpapers {
                            self.wallpaper_queue.push_back(wp);
                        }
                        self.status_message = None;
                        if self.current_wallpaper.is_none() && !self.wallpaper_queue.is_empty() {
                            return self.set_next_wallpaper();
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch wallpapers: {e}");
                        self.status_message = Some(e);
                    }
                }
            }

            Message::WallpaperReady(result) => match result {
                Ok(wp) => {
                    self.current_wallpaper = Some(wp.clone());
                    self.history.push(wp.clone());
                    self.history_index = self.history.len() - 1;
                    self.status_message = None;

                    if let Some(ref path) = wp.local_path {
                        if let Err(e) = background::set_wallpaper(path, &self.config.scaling_mode) {
                            tracing::error!("Failed to set wallpaper: {e}");
                            self.status_message = Some(format!("Failed to set wallpaper: {e}"));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to download wallpaper: {e}");
                    self.status_message = Some(e);
                }
            },

            Message::WallpaperSet(result) => {
                if let Err(e) = result {
                    tracing::error!("Failed to set wallpaper: {e}");
                    self.status_message = Some(e);
                }
            }

            Message::FetchMore => {
                if !self.is_fetching {
                    return self.fetch_wallpapers();
                }
            }

            Message::ToggleSourceBing => {
                self.config.source_bing = !self.config.source_bing;
                self.save_config();
            }
            Message::ToggleSourceNasa => {
                self.config.source_nasa = !self.config.source_nasa;
                self.save_config();
            }
            Message::ToggleSourceWallhaven => {
                self.config.source_wallhaven = !self.config.source_wallhaven;
                self.save_config();
            }
            Message::ToggleSourceEarthView => {
                self.config.source_earthview = !self.config.source_earthview;
                self.save_config();
            }
            Message::ToggleSourceLocal => {
                self.config.source_local = !self.config.source_local;
                self.save_config();
            }
            Message::SetRotationInterval(secs) => {
                self.config.rotation_interval_secs = secs;
                self.seconds_until_next = secs;
                self.save_config();
            }
            Message::SetIntervalHours(val) => {
                self.interval_hours = val;
                self.apply_interval_from_fields();
            }
            Message::SetIntervalMinutes(val) => {
                self.interval_minutes = val;
                self.apply_interval_from_fields();
            }
            Message::SetIntervalSeconds(val) => {
                self.interval_seconds = val;
                self.apply_interval_from_fields();
            }
            Message::SetScalingMode(mode) => {
                self.config.scaling_mode = mode;
                self.save_config();
                if let Some(ref wp) = self.current_wallpaper {
                    if let Some(ref path) = wp.local_path {
                        let _ = background::set_wallpaper(path, &self.config.scaling_mode);
                    }
                }
            }
            Message::SetThemeFilter(filter) => {
                self.config.theme_filter = filter;
                self.save_config();
            }
            Message::SetBrightnessThreshold(val) => {
                self.config.brightness_threshold = val;
                self.save_config();
            }
            Message::ConfigChanged(new_config) => {
                self.config = new_config;
            }

            Message::Tray(action) => match action {
                TrayAction::ShowWindow => {
                    // TODO: focus/show the main window
                }
                TrayAction::NextWallpaper => {
                    return self.set_next_wallpaper();
                }
                TrayAction::TogglePause => {
                    self.config.paused = !self.config.paused;
                    self.tray_paused
                        .store(self.config.paused, Ordering::Relaxed);
                    self.save_config();
                    if !self.config.paused {
                        self.seconds_until_next = self.config.rotation_interval_secs;
                        self.last_tick = Some(Instant::now());
                    }
                }
                TrayAction::Quit => {
                    std::process::exit(0);
                }
            },
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let active_page = self
            .nav_model
            .active_data::<Page>()
            .copied()
            .unwrap_or(Page::Sources);

        match active_page {
            Page::Sources => pages::sources::view(&self.config),
            Page::Schedule => pages::schedule::view(self),
            Page::Appearance => pages::appearance::view(&self.config),
            Page::History => {
                pages::history::view(&self.history, self.history_index, &self.config.favorites)
            }
        }
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let mut elements = Vec::new();

        if let Some(ref wp) = self.current_wallpaper {
            elements.push(
                widget::text::body(&wp.title)
                    .width(Length::Shrink)
                    .into(),
            );
        }

        elements
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let mut elements = Vec::new();

        elements.push(
            widget::button::icon(widget::icon::from_name("go-previous-symbolic"))
                .on_press(Message::PreviousWallpaper)
                .tooltip(fl!("action-previous"))
                .into(),
        );

        let pause_icon = if self.config.paused {
            "media-playback-start-symbolic"
        } else {
            "media-playback-pause-symbolic"
        };
        elements.push(
            widget::button::icon(widget::icon::from_name(pause_icon))
                .on_press(Message::TogglePause)
                .tooltip(if self.config.paused {
                    fl!("action-resume")
                } else {
                    fl!("action-pause")
                })
                .into(),
        );

        elements.push(
            widget::button::icon(widget::icon::from_name("go-next-symbolic"))
                .on_press(Message::NextWallpaper)
                .tooltip(fl!("action-next"))
                .into(),
        );

        if !self.config.paused {
            let mins = self.seconds_until_next / 60;
            let secs = self.seconds_until_next % 60;
            elements.push(widget::text::caption(format!("{mins}:{secs:02}")).into());
        }

        elements
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let timer = scheduler::timer_subscription(self.config.paused).map(Message::TimerTick);

        // Poll the tray action channel
        let tray_sub = Subscription::run(tray_subscription).map(Message::Tray);

        Subscription::batch([timer, tray_sub])
    }
}

fn tray_subscription() -> impl futures::Stream<Item = TrayAction> {
    stream::unfold(tray::take_receiver(), |mut rx| async move {
        let action = rx.recv().await?;
        Some((action, rx))
    })
}

impl Papery {
    fn apply_interval_from_fields(&mut self) {
        let h: u64 = self.interval_hours.parse().unwrap_or(0);
        let m: u64 = self.interval_minutes.parse().unwrap_or(0);
        let s: u64 = self.interval_seconds.parse().unwrap_or(0);
        let total = h * 3600 + m * 60 + s;
        if total > 0 {
            self.config.rotation_interval_secs = total;
            self.seconds_until_next = total;
            self.save_config();
        }
    }

    fn save_config(&self) {
        if let Some(ref handler) = self.config_handler {
            if let Err(e) = self.config.write_entry(handler) {
                tracing::error!("Failed to save config: {e:?}");
            }
        }
    }

    fn enabled_providers(&self) -> Vec<Box<dyn WallpaperProvider>> {
        let mut providers: Vec<Box<dyn WallpaperProvider>> = Vec::new();

        if self.config.source_bing {
            providers.push(Box::new(BingProvider));
        }
        if self.config.source_nasa {
            providers.push(Box::new(NasaApodProvider));
        }
        if self.config.source_wallhaven {
            providers.push(Box::new(WallhavenProvider::new(
                &self.config.wallhaven_categories,
                &self.config.wallhaven_purity,
            )));
        }
        if self.config.source_earthview {
            providers.push(Box::new(EarthViewProvider));
        }
        if self.config.source_local {
            let folders: Vec<PathBuf> = self
                .config
                .local_folders
                .iter()
                .map(PathBuf::from)
                .collect();
            providers.push(Box::new(LocalProvider::new(folders)));
        }

        providers
    }

    fn fetch_wallpapers(&mut self) -> app::Task<Message> {
        let providers = self.enabled_providers();
        if providers.is_empty() {
            self.status_message = Some(fl!("error-no-sources"));
            return Task::none();
        }

        self.is_fetching = true;

        cosmic::task::future(async move {
            let mut all_wallpapers = Vec::new();

            for provider in &providers {
                match provider.fetch_wallpapers(5).await {
                    Ok(wallpapers) => {
                        all_wallpapers.extend(wallpapers);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch from {}: {e}", provider.name());
                    }
                }
            }

            use rand::seq::SliceRandom;
            let mut rng = rand::rng();
            all_wallpapers.shuffle(&mut rng);

            if all_wallpapers.is_empty() {
                Message::WallpapersFetched(Err(fl!("error-network")))
            } else {
                Message::WallpapersFetched(Ok(all_wallpapers))
            }
        })
    }

    fn set_next_wallpaper(&mut self) -> app::Task<Message> {
        if let Some(mut wp) = self.wallpaper_queue.pop_front() {
            let cache_dir = self.download_manager.cache_dir.clone();
            let theme_filter = self.config.theme_filter.clone();
            let brightness_threshold = self.config.brightness_threshold;

            return cosmic::task::future(async move {
                let dm = DownloadManager::new(cache_dir);
                match dm.download(&mut wp).await {
                    Ok(path) => {
                        if theme_filter != "any" {
                            if let Ok(b) = tokio::task::spawn_blocking(move || {
                                brightness::analyze_brightness(&path)
                            })
                            .await
                            {
                                if let Ok(b) = b {
                                    wp.brightness = Some(b);
                                    if !brightness::matches_theme(
                                        b,
                                        brightness_threshold,
                                        &theme_filter,
                                    ) {
                                        return Message::WallpaperSet(Ok(()));
                                    }
                                }
                            }
                        }
                        Message::WallpaperReady(Ok(wp))
                    }
                    Err(e) => Message::WallpaperReady(Err(e.to_string())),
                }
            });
        }

        self.fetch_wallpapers()
    }

    fn apply_wallpaper(&self, wp: WallpaperInfo) -> app::Task<Message> {
        if let Some(ref path) = wp.local_path {
            match background::set_wallpaper(path, &self.config.scaling_mode) {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!("Failed to set wallpaper: {e}");
                }
            }
        }
        Task::none()
    }
}
