use anyhow::Result;
use iced_wgpu::{button, Renderer};
use iced_winit::{program, Command, Element, Length, Row, Text};
use winit::event::ModifiersState;

use crate::{
    fps::FpsCounter,
    input::InputState,
    settings::Settings,
    singleplayer::SinglePlayer,
    window::{State, StateFactory, StateTransition, WindowBuffers, WindowData, WindowFlags},
    render::iced::IcedRenderer,
};
use voxel_rs_common::network::dummy;
use voxel_rs_server::launch_server;


#[derive(Debug, Clone, Copy)]
pub enum Message {
    ResumeGame,
    ExitGame,
}

#[derive(Debug, Copy, Clone)]
pub struct PauseMenuControls {
    exit_button_state: button::State,
    pub should_exit: bool,
    resume_button_state: button::State,
    pub should_resume: bool,
}

impl PauseMenuControls {
    pub fn new() -> Self {
        PauseMenuControls {
            exit_button_state: button::State::new(),
            should_exit: false,
            resume_button_state: button::State::new(),
            should_resume: false,
        }
    }
}

impl program::Program for PauseMenuControls {
    type Renderer = iced_wgpu::Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        log::debug!("Received UI message: {:?}", message);
        match message {
            Message::ResumeGame => self.should_resume = true,
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
                    &mut self.resume_button_state,
                    Text::new("Resume Game"),
                )
                .on_press(Message::ResumeGame),
            )
            .push(
                button::Button::new(&mut self.exit_button_state, Text::new("Exit Game"))
                    .on_press(Message::ExitGame),
            )
            .into()
    }
}
