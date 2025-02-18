use crate::PocoWM;
use anyhow::{anyhow, Context as _, Result};
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent};
use smithay::desktop::space::render_output;
use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
use smithay::utils::{Rectangle, Transform};
use std::time::Duration;

impl PocoWM {
    pub fn init_winit(&mut self) -> Result<()> {
        let (mut backend, winit) = winit::init::<GlesRenderer>()
            .map_err(|err| anyhow!("{:#?}", err))
            .context("Failed to init winit")?;

        let mode = Mode {
            size: backend.window_size(),
            refresh: 60_000,
        };

        let output = Output::new(
            "winit".into(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "PocoWM".into(),
                model: "Winit".into(),
            },
        );
        output.create_global::<PocoWM>(&self.display);
        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );
        output.set_preferred(mode);
        self.renderer.map_output(&output, (0, 0));

        let mut damage_tracker = OutputDamageTracker::from_output(&output);

        std::env::set_var("WAYLAND_DISPLAY", &self.socket_name);

        self.event_loop
            .borrow_mut()
            .handle()
            .insert_source(winit, move |event, _, state| {
                let result = (|| {
                    match event {
                        WinitEvent::Resized { size, .. } => {
                            output.change_current_state(
                                Some(Mode {
                                    size,
                                    refresh: 60_000,
                                }),
                                None,
                                None,
                                None,
                            );
                            state.renderer.render(&state.layout);
                        }
                        WinitEvent::Input(event) => {
                            state.handle_input(event);
                        }
                        WinitEvent::Redraw => {
                            let damage = Rectangle::from_size(backend.window_size());
                            backend.bind().context("Failed to bind winit")?;
                            render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
                                &output,
                                backend.renderer(),
                                1.0,
                                0,
                                [&state.renderer.space],
                                &[],
                                &mut damage_tracker,
                                // [0.1, 0.1, 0.1, 1.0],
                                [0.8, 0.8, 0.8, 1.0],
                            )
                            .context("Failed to render output")?;
                            backend
                                .submit(Some(&[damage]))
                                .context("Failed to submit")?;

                            state.layout.iter_windows().for_each(|window| {
                                window.send_frame(
                                    &output,
                                    state.start_time.elapsed(),
                                    Some(Duration::ZERO),
                                    |_, _| Some(output.clone()),
                                )
                            });

                            state.renderer.refresh();
                            state.popups.cleanup();
                            let _ = state.display.flush_clients();
                            backend.window().request_redraw();
                        }
                        WinitEvent::CloseRequested => {
                            state.loop_signal.stop();
                        }
                        _ => (),
                    }
                    Ok::<_, anyhow::Error>(())
                })();
                if let Err(err) = result {
                    eprintln!("{:?}", err);
                }
            })
            .map_err(|err| anyhow!(err.to_string()))
            .context("Failed to insert winit source")?;

        Ok(())
    }
}
