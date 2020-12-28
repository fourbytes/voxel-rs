use anyhow::Result;
use iced_wgpu::{button, Renderer};
use iced_winit::{conversion, program, Command, Element, Length, Row, Size, Text, Viewport};
use winit::event::ModifiersState;

use crate::{
    fps::FpsCounter,
    input::InputState,
    settings::Settings,
    singleplayer::SinglePlayer,
    window::{State, StateFactory, StateTransition, WindowBuffers, WindowData, WindowFlags},
};
use voxel_rs_common::network::dummy;
use voxel_rs_server::launch_server;

/// State of the main menu
pub struct MainMenu {
    fps_counter: FpsCounter,
    ui_renderer: iced_wgpu::Renderer,
    state: program::State<MainMenuControls>,
    window_data: Option<WindowData>,
    cursor_position: iced_winit::winit::dpi::PhysicalPosition<f64>,
}

impl MainMenu {
    pub fn new_factory() -> crate::window::StateFactory {
        Box::new(move |settings, device| Self::new(settings, device))
    }

    pub fn new(
        settings: &mut Settings,
        device: &mut wgpu::Device,
    ) -> Result<(Box<dyn State>, wgpu::CommandBuffer)> {
        log::info!("Initializing main menu");

        // Create the renderers
        let mut ui_renderer = iced_wgpu::Renderer::new(iced_wgpu::Backend::new(
            device,
            iced_wgpu::Settings {
                format: crate::window::COLOR_FORMAT,
                present_mode: crate::window::PRESENT_MODE,
                default_font: None,
                default_text_size: 50,
                antialiasing: None,
            },
        ));

        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("main_menu_encoder"),
        });

        let viewport = Viewport::with_physical_size(
            Size::new(
                settings.window_size[0] as u32,
                settings.window_size[1] as u32,
            ),
            1.0,
        );
        let mut debug = iced_winit::Debug::new();
        let cursor_position = iced_winit::winit::dpi::PhysicalPosition::new(-1.0, -1.0);
        let controls = MainMenuControls::new();
        let state = program::State::new(
            controls,
            viewport.logical_size(),
            conversion::cursor_position(cursor_position.into(), viewport.scale_factor()),
            &mut ui_renderer,
            &mut debug,
        );

        Ok((
            Box::new(Self {
                fps_counter: FpsCounter::new(),
                ui_renderer,
                cursor_position,
                state,
                window_data: None,
            }),
            encoder.finish(),
        ))
    }

    fn start_single_player(&mut self) -> Box<StateFactory> {
        let (client, server) = dummy::new();

        std::thread::spawn(move || {
            if let Err(e) = launch_server(Box::new(server)) {
                // TODO: rewrite this error reporting
                log::error!(
                    "Error happened in the server code: {}\nPrinting chain:\n{}",
                    e,
                    e.chain()
                        .enumerate()
                        .map(|(i, e)| format!("{}: {}", i, e))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
        });

        Box::new(SinglePlayer::new_factory(Box::new(client)))
    }
}

impl State for MainMenu {
    fn update(
        &mut self,
        _settings: &mut Settings,
        input_state: &InputState,
        _data: &WindowData,
        flags: &mut WindowFlags,
        _seconds_delta: f64,
        _device: &mut wgpu::Device,
    ) -> Result<StateTransition> {
        flags.grab_cursor = false;

        if self.state.program().should_exit {
            Ok(StateTransition::CloseWindow)
        } else if self.state.program().should_start_single_player {
            Ok(StateTransition::ReplaceCurrent(self.start_single_player()))
        } else {
            Ok(StateTransition::KeepCurrent)
        }
    }

    fn render<'a>(
        &mut self,
        settings: &Settings,
        buffers: WindowBuffers<'a>,
        device: &mut wgpu::Device,
        data: &WindowData,
        input_state: &InputState,
    ) -> Result<(StateTransition, wgpu::CommandBuffer)> {
        self.window_data = Some(data.clone());
        let viewport = Viewport::with_physical_size(
            Size::new(
                data.physical_window_size.width,
                data.physical_window_size.height,
            ),
            data.scale_factor,
        );
        self.state.update(
            viewport.logical_size(),
            conversion::cursor_position(self.cursor_position.into(), viewport.scale_factor()),
            None,
            &mut self.ui_renderer,
            &mut iced_winit::Debug::new(),
        );
        self.fps_counter.add_frame();

        // Initialize encoder and clear buffers.
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        crate::render::clear_color_and_depth(&mut encoder, buffers);

        // Rebuild ui
        let mut staging_belt = wgpu::util::StagingBelt::new(128);
        self.ui_renderer.backend_mut().draw(
            device,
            &mut staging_belt,
            &mut encoder,
            buffers.texture_buffer,
            &viewport,
            self.state.primitive(),
            &["Main Menu"],
        );

        staging_belt.finish();

        Ok((StateTransition::KeepCurrent, encoder.finish()))
    }

    fn handle_window_event(&mut self, event: winit::event::WindowEvent, input_state: &InputState) {
        // Map window event to iced event
        if let Some(event) = iced_winit::conversion::window_event(
            &event,
            self.window_data
                .as_ref()
                .map(|w| w.scale_factor)
                .unwrap_or(1.0),
            input_state._get_modifiers_state(),
        ) {
            self.state.queue_event(event);
        }
    }

    fn handle_mouse_motion(&mut self, _: &Settings, _: (f64, f64)) {}

    fn handle_cursor_movement(&mut self, logical_position: winit::dpi::LogicalPosition<f64>) {
        let (x, y) = logical_position
            .to_physical::<f64>(
                self.window_data
                    .as_ref()
                    .map(|w| w.scale_factor)
                    .unwrap_or(1.0),
            )
            .into();
        self.cursor_position = iced_winit::winit::dpi::PhysicalPosition::new(x, y)
    }

    fn handle_mouse_state_changes(
        &mut self,
        changes: Vec<(winit::event::MouseButton, winit::event::ElementState)>,
    ) {
        //self.ui.handle_mouse_state_changes(changes);
    }

    fn handle_key_state_changes(&mut self, changes: Vec<(u32, winit::event::ElementState)>) {
        //self.ui.handle_key_state_changes(changes);
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    StartSinglePlayer,
    ExitGame,
}

#[derive(Debug, Copy, Clone)]
struct MainMenuControls {
    exit_button_state: button::State,
    pub(self) should_exit: bool,
    start_single_player_button_state: button::State,
    pub(self) should_start_single_player: bool,
}

impl MainMenuControls {
    pub fn new() -> Self {
        MainMenuControls {
            exit_button_state: button::State::new(),
            should_exit: false,
            start_single_player_button_state: button::State::new(),
            should_start_single_player: false,
        }
    }
}

impl program::Program for MainMenuControls {
    type Renderer = iced_wgpu::Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        log::debug!("Received UI message: {:?}", message);
        match message {
            Message::StartSinglePlayer => self.should_start_single_player = true,
            Message::ExitGame => self.should_exit = true,
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        Row::new()
            .width(Length::Units(500))
            .spacing(20)
            .push(
                button::Button::new(
                    &mut self.start_single_player_button_state,
                    Text::new("Single Player"),
                )
                .on_press(Message::StartSinglePlayer),
            )
            .push(
                button::Button::new(&mut self.exit_button_state, Text::new("Exit Game"))
                    .on_press(Message::ExitGame),
            )
            .into()
    }
}
