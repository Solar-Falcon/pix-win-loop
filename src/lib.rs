#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![warn(missing_docs)]

/*
Possible TODO:
* add PixelsBuilder customization
*/

use cfg_if::cfg_if;
use pixels::SurfaceTexture;
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::Window,
};

use web_time::Instant;

mod error;
pub use error::{Error, Result};

mod input;
pub use input::{Input, InputState};

#[doc(no_inline)]
pub use pixels;
#[doc(no_inline)]
pub use winit;

#[doc(no_inline)]
pub use pixels::Pixels;
#[doc(no_inline)]
pub use web_time::Duration;
#[doc(no_inline)]
pub use winit::dpi::PhysicalSize;
#[doc(no_inline)]
pub use winit::keyboard::{KeyCode, NamedKey};
#[doc(no_inline)]
pub use winit::window::WindowBuilder;

/// Update context.
#[derive(Debug)]
pub struct Context {
    target_frame_time: Duration,
    max_frame_time: Duration,
    exit: bool,
    delta_time: Duration,
    /// Input handler.
    pub input: Input,
}

impl Context {
    #[inline]
    fn new(window: Rc<Window>, target_frame_time: Duration, max_frame_time: Duration) -> Self {
        Self {
            target_frame_time,
            max_frame_time,
            delta_time: Duration::ZERO,
            exit: false,
            input: Input::new(window),
        }
    }

    /// `winit` window.
    #[inline]
    pub fn window(&self) -> &Window {
        &self.input.window
    }

    /// Time between previous and current update.
    #[inline]
    pub fn frame_time(&self) -> Duration {
        self.delta_time
    }

    /// Set the desired (minimum) time between application updates.
    /// Implemented based on <https://gafferongames.com/post/fix_your_timestep>.
    #[inline]
    pub fn set_target_frame_time(&mut self, time: Duration) {
        self.target_frame_time = time;
    }

    /// Set the maximum time between application updates.
    /// The real frame time can be loger, but [`Context::frame_time()`] will not exceed this value.
    /// Implemented based on <https://gafferongames.com/post/fix_your_timestep>.
    #[inline]
    pub fn set_max_frame_time(&mut self, time: Duration) {
        self.max_frame_time = time;
    }

    /// Exit the application.
    #[inline]
    pub fn exit(&mut self) {
        self.exit = true;
    }
}

/// Application trait
pub trait App {
    /// Application update.
    /// Rate of updates can be set using [`Context`].
    fn update(&mut self, ctx: &mut Context) -> Result<()>;

    /// Application render.
    /// Will be called once every frame.
    fn render(&mut self, pix: &mut Pixels) -> Result<()>;

    /// Custom event handler if needed.
    #[inline]
    fn handle(&mut self, _event: &Event<()>) -> Result<()> {
        Ok(())
    }
}

/// Start the application. Not available on web target.
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
pub async fn start_async(
    #[allow(unused_mut)] mut window_builder: WindowBuilder,
    mut app: impl App + 'static,
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

    let window = Rc::new(window_builder.build(&event_loop)?);
    let window_size = window.inner_size();

    let mut context = Context::new(window.clone(), target_frame_time, max_frame_time);
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &*window);
    let mut pixels = Pixels::new_async(
        pixel_buffer_size.width,
        pixel_buffer_size.height,
        surface_texture,
    )
    .await?;

    let mut instant = Instant::now();
    let mut accumulated_time = Duration::ZERO;

    event_loop.set_control_flow(ControlFlow::Poll);

    let event_handler = move |event: Event<()>, elwt: &EventLoopWindowTarget<()>| {
        elwt.set_control_flow(ControlFlow::Poll);

        if handle_error(app.handle(&event), elwt).is_err() {
            return;
        }

        context.input.process_event(&event);

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(new_size) => {
                        if handle_error(
                            pixels
                                .resize_surface(new_size.width, new_size.height)
                                .map_err(Into::into),
                            elwt,
                        )
                        .is_err()
                        {
                            #[allow(clippy::needless_return)]
                            // keep 'return' in case I add code after this
                            return;
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        let mut elapsed = instant.elapsed();
                        instant = Instant::now();

                        if elapsed > context.max_frame_time {
                            elapsed = context.max_frame_time;
                        }

                        accumulated_time += elapsed;

                        let mut keys_updated = false;

                        while accumulated_time > context.target_frame_time {
                            context.delta_time = context.target_frame_time;

                            if handle_error(app.update(&mut context), elwt).is_err() {
                                return;
                            }

                            if context.exit {
                                elwt.exit();
                                return;
                            }

                            if !keys_updated {
                                context.input.update_keys();
                                keys_updated = true;
                            }

                            accumulated_time =
                                accumulated_time.saturating_sub(context.target_frame_time);
                        }

                        // blending_factor = accumulated_time.as_secs_f32() / context.target_frame_time.as_secs_f32();

                        if handle_error(app.render(&mut pixels), elwt).is_err() {
                            return;
                        }

                        if handle_error(pixels.render().map_err(Into::into), elwt).is_err() {
                            #[allow(clippy::needless_return)]
                            // keep 'return' in case I add code after this and don't notice
                            return;
                        }
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    };

    cfg_if! {
        if #[cfg(all(target_arch = "wasm32", feature = "winit-event-loop-spawn"))] {
            use winit::platform::web::EventLoopExtWebSys;

            event_loop.spawn(event_handler);
        } else {
            event_loop.run(event_handler)?;
        }
    }

    Ok(())
}

#[inline]
fn handle_error(
    result: Result<()>,
    elwt: &EventLoopWindowTarget<()>,
) -> std::result::Result<(), ()> {
    if let Err(error) = result {
        log::error!("{}", error);
        elwt.exit();

        Err(())
    } else {
        Ok(())
    }
}
