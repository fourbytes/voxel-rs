use iced_native::{
    program::{Program, State},
    Point, Size,
};
use iced_wgpu::{Backend, Renderer, Viewport};
use iced_winit::Debug;
use wgpu::Device;
use winit::{dpi::LogicalPosition, event::ModifiersState};

use crate::window::{WindowBuffers, WindowData, COLOR_FORMAT, PRESENT_MODE};

fn viewport_from_window_data(window_data: &WindowData) -> Viewport {
    Viewport::with_physical_size(
        Size::new(
            window_data.physical_window_size.width,
            window_data.physical_window_size.height,
        ),
        window_data.scale_factor,
    )
}

pub struct IcedRenderer<P, M>
where
    P: 'static + Program<Message = M, Renderer = Renderer>,
    M: Send + Copy + Clone + std::fmt::Debug,
{
    pub renderer: Renderer,
    pub viewport: Viewport,
    pub modifiers_state: ModifiersState,
    pub cursor_position: winit::dpi::LogicalPosition<f64>,
    pub debug: Debug,
    pub state: State<P>,
}

impl<P, M> IcedRenderer<P, M>
where
    P: 'static + Program<Message = M, Renderer = Renderer>,
    M: Send + Copy + Clone + std::fmt::Debug,
{
    pub fn new(
        program: P,
        device: &mut Device,
        window_data: &WindowData,
        modifiers_state: &ModifiersState,
    ) -> Self {
        let viewport = viewport_from_window_data(window_data);
        let mut debug = iced_winit::Debug::new();
        let cursor_position = LogicalPosition::new(-1.0, -1.0);

        let mut renderer = Renderer::new(Backend::new(
            device,
            iced_wgpu::Settings {
                format: COLOR_FORMAT,
                present_mode: PRESENT_MODE,
                default_font: Some(include_bytes!(
                    "../../../assets/fonts/IBMPlexMono-SemiBold.ttf"
                )),
                default_text_size: 40,
                antialiasing: None,
            },
        ));

        Self {
            state: State::new(
                program,
                viewport.logical_size(),
                Point::new(cursor_position.x as f32, cursor_position.y as f32),
                &mut renderer,
                &mut debug,
            ),
            cursor_position,
            renderer,
            viewport,
            modifiers_state: modifiers_state.clone(),
            debug,
        }
    }
    pub fn update(&mut self, window_data: &WindowData) {
        self.viewport = viewport_from_window_data(window_data);

        self.state.update(
            self.viewport.logical_size(),
            Point::new(self.cursor_position.x as f32, self.cursor_position.y as f32),
            None,
            &mut self.renderer,
            &mut self.debug,
        );
    }

    pub fn handle_window_event(&mut self, event: winit::event::WindowEvent) {
        if let Some(event) = iced_winit::conversion::window_event(
            &event,
            self.viewport.scale_factor(),
            self.modifiers_state,
        ) {
            self.state.queue_event(event);
        }
    }

    pub fn handle_cursor_movement(&mut self, logical_position: winit::dpi::LogicalPosition<f64>) {
        self.cursor_position = logical_position;
    }

    pub fn render<'a>(
        &mut self,
        device: &mut Device,
        buffers: WindowBuffers<'a>,
        mut encoder: &mut wgpu::CommandEncoder,
        debug_info: Option<Vec<String>>,
    ) {
        crate::render::clear_color_and_depth(&mut encoder, buffers);

        let mut staging_belt = wgpu::util::StagingBelt::new(128);
        self.renderer.backend_mut().draw(
            device,
            &mut staging_belt,
            &mut encoder,
            buffers.texture_buffer,
            &self.viewport,
            self.state.primitive(),
            &debug_info.unwrap_or(vec![]),
        );

        staging_belt.finish();
    }
}
