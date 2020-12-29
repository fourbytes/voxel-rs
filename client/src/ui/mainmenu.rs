use anyhow::Result;
use iced_wgpu::{button, Renderer};
use iced_winit::{program, Align, Column, Command, Element, HorizontalAlignment, Length, Text};
use winit::event::ModifiersState;
use winit::event::{VirtualKeyCode};

use crate::{
    fps::FpsCounter,
    input::InputState,
    render::iced::IcedRenderer,
    settings::Settings,
    singleplayer::SinglePlayer,
    window::{State, StateFactory, StateTransition, WindowBuffers, WindowData, WindowFlags},
};
use voxel_rs_common::network::dummy;
use voxel_rs_server::launch_server;

/// State of the main menu
pub struct MainMenu {
    fps_counter: FpsCounter,
    ui_renderer: IcedRenderer<MainMenuControls, Message>,
}

impl MainMenu {
    pub fn new_factory() -> crate::window::StateFactory {
        Box::new(move |device, _settings, window_data, modifiers_state| {
            Self::new(device, window_data, modifiers_state)
        })
    }

    pub fn new(
        device: &mut wgpu::Device,
        window_data: &WindowData,
        modifiers_state: &ModifiersState,
    ) -> Result<(Box<dyn State>, wgpu::CommandBuffer)> {
        log::info!("Initializing main menu");

        // Create the renderers
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("main_menu_encoder"),
        });
        let ui_renderer = IcedRenderer::new(
            MainMenuControls::new(),
            device,
            window_data,
            modifiers_state,
        );

        Ok((
            Box::new(Self {
                fps_counter: FpsCounter::new(),
                ui_renderer,
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
        _input_state: &InputState,
        _data: &WindowData,
        flags: &mut WindowFlags,
        _seconds_delta: f64,
        _device: &mut wgpu::Device,
    ) -> Result<StateTransition> {
        flags.grab_cursor = false;

        if self.ui_renderer.state.program().should_exit {
            Ok(StateTransition::CloseWindow)
        } else if self.ui_renderer.state.program().should_start_single_player {
            Ok(StateTransition::ReplaceCurrent(self.start_single_player()))
        } else {
            Ok(StateTransition::KeepCurrent)
        }
    }

    fn render<'a>(
        &mut self,
        _settings: &Settings,
        buffers: WindowBuffers<'a>,
        device: &mut wgpu::Device,
        window_data: &WindowData,
        _input_state: &InputState,
    ) -> Result<(StateTransition, wgpu::CommandBuffer)> {
        self.fps_counter.add_frame();
        self.ui_renderer.update(window_data);

        // Initialize encoder and clear buffers.
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        crate::render::clear_color_and_depth(&mut encoder, buffers);

        // Render Iced UI
        self.ui_renderer.render(device, buffers, &mut encoder, None);

        Ok((StateTransition::KeepCurrent, encoder.finish()))
    }

    fn handle_window_event(&mut self, event: winit::event::WindowEvent, _input_state: &InputState) {
        self.ui_renderer.handle_window_event(event);
    }

    fn handle_cursor_movement(&mut self, logical_position: winit::dpi::LogicalPosition<f64>) {
        self.ui_renderer.handle_cursor_movement(logical_position);
    }

    fn handle_mouse_motion(&mut self, _: &Settings, _: (f64, f64)) {}

    fn handle_mouse_state_changes(
        &mut self,
        _: Vec<(winit::event::MouseButton, winit::event::ElementState)>,
    ) {
    }

    fn handle_key_state_changes(&mut self, _: Vec<(VirtualKeyCode, winit::event::ElementState)>) {}
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
        Column::new()
            .padding(60)
            .width(Length::Fill)
            .align_items(Align::Center)
            .spacing(20)
            .push(
                button::Button::new(
                    &mut self.start_single_player_button_state,
                    Text::new("Single Player")
                        .size(30)
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .width(Length::Units(300))
                .on_press(Message::StartSinglePlayer),
            )
            .push(
                button::Button::new(
                    &mut self.exit_button_state,
                    Text::new("Exit Game")
                        .size(30)
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .width(Length::Units(300))
                .on_press(Message::ExitGame),
            )
            .into()
    }
}
