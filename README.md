# pix-win-loop

GPU pixel buffer (using [`pixels`][1]), windowing (using [`winit`][2]), nice input handling and frame-rate-independent game loop all wrapped up in a neat little package.
The game loop is based on <https://gafferongames.com/post/fix_your_timestep>.

## Small example

```no_run
use pix_win_loop::*;

struct Application;

impl App for Application {
    fn update(&mut self, ctx: &mut Context) -> Result<()> {
        if ctx.input.is_logical_key_pressed(NamedKey::Escape) {
            ctx.exit();
        }

        Ok(())
    }

    fn render(&mut self, pixels: &mut Pixels) -> Result<()> {
        // do rendering using pixels.

        let mut frame = pixels.frame_mut();

        // draw a 400x12 green line
        for pixel in frame.chunks_exact_mut(4).take(PIX_WIDTH as usize * 12) {
            pixel[1] = 255;
            pixel[3] = 255;
        }

        Ok(())
    }
}

const PIX_WIDTH: u32 = 400;
const PIX_HEIGHT: u32 = 300;

fn main() -> Result<()> {
    let window_builder = WindowBuilder::new()
        .with_title("win-pix-loop example")
        .with_inner_size(PhysicalSize::new(800, 600));

    // Pixel buffer will be scaled to the window size.
    // So e.g. this will result in 2x scaling.
    let pixel_buffer_size = PhysicalSize::new(PIX_WIDTH, PIX_HEIGHT);

    // Minimum time between updates.
    // See https://gafferongames.com/post/fix_your_timestep.
    let target_frame_time = Duration::from_secs_f32(1. / 120.); // 120 fps

    // Maximum time between updates.
    // The real time can still exceed this value.
    // See https://gafferongames.com/post/fix_your_timestep.
    let max_frame_time = Duration::from_secs_f32(0.1);

    pix_win_loop::start(window_builder, Application, pixel_buffer_size, target_frame_time, max_frame_time)
}

```

## Features

* `winit-event-loop-spawn`: on web targets uses [`EventLoop::spawn()`][4] instead of [`EventLoop::run()`][3] for the main loop.

[1]: https://crates.io/crates/pixels
[2]: https://crates.io/crates/winit
[3]: https://docs.rs/winit/0.29.5/winit/event_loop/struct.EventLoop.html#method.run
[4]: https://docs.rs/winit/0.29.5/wasm32-unknown-unknown/winit/platform/web/trait.EventLoopExtWebSys.html#tymethod.spawn
