// SPDX-License-Identifier: {{LICENSE}}

use crate::config::Config;
use crate::core::nav::NavPage;
use crate::{fl, pages};
use cosmic::app::{Command, Core};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::time;
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, menu, nav_bar};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Element};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const REPOSITORY: &str = "https://github.com/pop-os/cosmic-app-template";
const APP_ICON: &[u8] =
    include_bytes!("../res/icons/hicolor/scalable/apps/com.github.francesco-gaglione.Cronos.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: Config,

    pub pomodoro: pages::pomodoro::Pomodoro,
    pub settings: pages::settings::Settings,

    pomodoro_tick_state: PomodoroTickState,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),

    // pages
    Pomodoro(pages::pomodoro::PomodoroMessage),
    Settings(pages::settings::SettingsMessage),

    PomodoroTick,
    StartPomodoroTimer,
    PausePomodoroTimer,
}

#[derive(Default)]
enum PomodoroTickState {
    #[default]
    Idle,
    Ticking {
        last_tick: Instant,
    },
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.francesco-gaglione.Cronos";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        // Create a nav bar with three page items.
        let mut nav = nav_bar::Model::default();

        for &nav_page in NavPage::all() {
            let id = nav
                .insert()
                .icon(nav_page.icon())
                .text(nav_page.title())
                .data::<NavPage>(nav_page)
                .id();

            if nav_page == NavPage::default() {
                nav.activate(id);
            }
        }

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    }
                })
                .unwrap_or_default(),
            pomodoro: pages::pomodoro::Pomodoro::default(),
            settings: pages::settings::Settings::default(),
            pomodoro_tick_state: PomodoroTickState::Idle,
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<Element<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => self.about(),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let spacing = cosmic::theme::active().cosmic().spacing;
        let entity = self.nav.active();
        let nav_page = self.nav.data::<NavPage>(entity).unwrap_or_default();

        widget::column::with_children(vec![nav_page.view(self)])
            .padding(spacing.space_xs)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        let tick = match self.pomodoro_tick_state {
            PomodoroTickState::Idle => Subscription::none(),
            PomodoroTickState::Ticking { .. } => {
                time::every(Duration::from_secs(1)).map(|_instant| Message::PomodoroTick)
            }
        };
        Subscription::batch(vec![tick])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Commands may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        let mut commands = vec![];
        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }

            Message::SubscriptionChannel => {
                // For example purposes only.
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }

                // Set the title of the context drawer.
                self.set_context_title(context_page.title());
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::Pomodoro(pomodoro_message) => commands.push(
                self.pomodoro
                    .update(pomodoro_message)
                    .map(cosmic::app::Message::App),
            ),
            Message::Settings(settings_message) => commands.push(
                self.settings
                    .update(settings_message)
                    .map(cosmic::app::Message::App),
            ),
            Message::PomodoroTick => {
                commands.push(
                    self.pomodoro
                        .update(pages::pomodoro::PomodoroMessage::PomodoroTick)
                        .map(cosmic::app::Message::App),
                );
            }
            Message::StartPomodoroTimer => {
                self.pomodoro_tick_state = PomodoroTickState::Ticking {
                    last_tick: Instant::now(),
                };
            }
            Message::PausePomodoroTimer => {
                self.pomodoro_tick_state = PomodoroTickState::Idle;
            }
        }
        Command::batch(commands)
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Command<Self::Message> {
        // Activate the page in the model.
        self.nav.activate(id);

        self.update_title()
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

        let title = widget::text::title3(fl!("app-title"));

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .align_items(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Command<Message> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        self.set_window_title(window_title)
    }
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

impl ContextPage {
    fn title(&self) -> String {
        match self {
            Self::About => fl!("about"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}
