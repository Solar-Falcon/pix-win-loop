#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![warn(missing_docs)]

pub use pixels;
pub use pixels::Pixels;
pub use win_loop::anyhow::{Error, Result};
pub use win_loop::winit::{
    self,
    dpi::PhysicalSize,
    keyboard::{KeyCode, NamedKey},
    window::WindowBuilder,
};
pub use win_loop::{Context, Duration, Input, InputState};

use pixels::SurfaceTexture;
use win_loop::winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowId,
};

/// Application trait.
pub trait App {
    /// Application update.
    /// Rate of updates can be set using [`Context`].
    fn update(&mut self, ctx: &mut Context) -> Result<()>;

    /// Application render.
    /// Will be called once every frame.
    fn render(&mut self, pix: &mut Pixels, blending_factor: f64) -> Result<()>;

    /// Custom event handler if needed.
    #[inline]
    fn handle(&mut self, _event: &Event<()>) -> Result<()> {
        Ok(())
    }
}

/// Start the application. Not available on web targets (use [`start_async()`]).
#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub fn start(
    window_builder: WindowBuilder,
    app: impl App + 'static,
    pixel_buffer_size: PhysicalSize<u32>,
    target_frame_time: Duration,
    max_frame_time: Duration,
) -> Result<()> {
    use pollster::FutureExt;

    start_async(
        window_builder,
        app,
        pixel_buffer_size,
        target_frame_time,
        max_frame_time,
    )
    .block_on()
}

/// Start the application asynchronously.
#[inline]
pub async fn start_async(
    #[allow(unused_mut)] mut window_builder: WindowBuilder,
    app: impl App + 'static,
    pixel_buffer_size: PhysicalSize<u32>,
    target_frame_time: Duration,
    max_frame_time: Duration,
) -> Result<()> {
    let event_loop = EventLoop::new()?;

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowBuilderExtWebSys;

        window_builder = window_builder.with_append(true);
    }

    let window = window_builder.build(&event_loop)?;
    let window_size = window.inner_size();

    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let pixels = Pixels::new_async(
        pixel_buffer_size.width,
        pixel_buffer_size.height,
        surface_texture,
    )
    .await?;

    let app = WinLoopApp {
        app,
        win_id: window.id(),
        resize_order: None,
    };

    win_loop::start(
        event_loop,
        window,
        app,
        pixels,
        target_frame_time,
        max_frame_time,
    )
}

struct WinLoopApp<A: App> {
    app: A,
    win_id: WindowId,
    resize_order: Option<PhysicalSize<u32>>,
}

impl<A> win_loop::App for WinLoopApp<A>
where
    A: App,
{
    type RenderContext = Pixels;

    #[inline]
    fn update(&mut self, ctx: &mut Context) -> Result<()> {
        self.app.update(ctx)
    }

    #[inline]
    fn render(&mut self, ctx: &mut Self::RenderContext, blending_factor: f64) -> Result<()> {
        if let Some(new_size) = self.resize_order {
            ctx.resize_surface(new_size.width, new_size.height)?;

            self.resize_order = None;
        }

        self.app.render(ctx, blending_factor)
    }

    #[inline]
    fn handle(&mut self, event: &Event<()>) -> Result<()> {
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(new_size),
            } if *window_id == self.win_id => {
                self.resize_order = Some(*new_size);
            }
            _ => {}
        }

        self.app.handle(event)
    }
}
