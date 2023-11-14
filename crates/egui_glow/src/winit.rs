use crate::shader_version::ShaderVersion;
use egui::{ViewportId, ViewportIdPair};
pub use egui_winit;
use egui_winit::winit;
pub use egui_winit::EventResponse;

/// Use [`egui`] from a [`glow`] app based on [`winit`].
pub struct EguiGlow {
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,
    pub painter: crate::Painter,

    shapes: Vec<egui::epaint::ClippedShape>,
    textures_delta: egui::TexturesDelta,
}

impl EguiGlow {
    /// For automatic shader version detection set `shader_version` to `None`.
    pub fn new<E>(
        event_loop: &winit::event_loop::EventLoopWindowTarget<E>,
        gl: std::sync::Arc<glow::Context>,
        shader_version: Option<ShaderVersion>,
        native_pixels_per_point: Option<f32>,
    ) -> Self {
        let painter = crate::Painter::new(gl, "", shader_version)
            .map_err(|err| {
                log::error!("error occurred in initializing painter:\n{err}");
            })
            .unwrap();

        Self {
            egui_ctx: Default::default(),
            egui_winit: egui_winit::State::new(
                event_loop,
                native_pixels_per_point,
                Some(painter.max_texture_side()),
            ),
            painter,
            shapes: Default::default(),
            textures_delta: Default::default(),
        }
    }

    pub fn on_event(&mut self, event: &winit::event::WindowEvent<'_>) -> EventResponse {
        self.egui_winit.on_event(&self.egui_ctx, event)
    }

    /// Call [`Self::paint`] later to paint.
    pub fn run(&mut self, window: &winit::window::Window, run_ui: impl FnMut(&egui::Context)) {
        let raw_input = self
            .egui_winit
            .take_egui_input(window, ViewportIdPair::ROOT);

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            viewports,
            viewport_commands,
        } = self.egui_ctx.run(raw_input, run_ui);

        if viewports.len() > 1 {
            log::warn!("Multiple viewports not yet supported by EguiGlow");
        }
        egui_winit::process_viewport_commands(
            viewport_commands.into_iter().map(|(_id, command)| command),
            window,
            true,
        );

        self.egui_winit.handle_platform_output(
            window,
            ViewportId::ROOT,
            &self.egui_ctx,
            platform_output,
        );

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self, window: &winit::window::Window) {
        let shapes = std::mem::take(&mut self.shapes);
        let mut textures_delta = std::mem::take(&mut self.textures_delta);

        for (id, image_delta) in textures_delta.set {
            self.painter.set_texture(id, &image_delta);
        }

        let pixels_per_point = self.egui_ctx.pixels_per_point();
        let clipped_primitives = self.egui_ctx.tessellate(shapes, pixels_per_point);
        let dimensions: [u32; 2] = window.inner_size().into();
        self.painter
            .paint_primitives(dimensions, pixels_per_point, &clipped_primitives);

        for id in textures_delta.free.drain(..) {
            self.painter.free_texture(id);
        }
    }

    /// Call to release the allocated graphics resources.
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}
