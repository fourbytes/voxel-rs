use iced_wgpu::{button, Renderer};
use iced_winit::{
    program::Program, Align, Column, Command, Element, HorizontalAlignment, Length, Text,
};

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

impl Program for PauseMenuControls {
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
        Column::new()
            .padding(60)
            .width(Length::Fill)
            .align_items(Align::Center)
            .spacing(20)
            .push(
                button::Button::new(
                    &mut self.resume_button_state,
                    Text::new("Resume Game")
                        .size(30)
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .width(Length::Units(300))
                .on_press(Message::ResumeGame),
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
