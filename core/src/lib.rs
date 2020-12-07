#[macro_use]
extern crate rustpython_vm;

mod anim;
mod prelude;
mod pyqs;
mod resources;

use anyhow::Context;
use rustpython_vm::{bytecode::FrozenModule, PySettings};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::prelude::*;

struct PickItUp {
    interp: Interpreter,
    sprites: RefCell<Resources>,

    update_fn: Option<PyObjectRef>,
    draw_fn: Option<PyObjectRef>,
    onload_fn: Option<PyObjectRef>,
    event_fn: Option<PyObjectRef>,
    state: PyObjectRef,
    last_update: Instant,

    window_initialized: bool,
}

fn handle_err<C>(vm: &VirtualMachine, e: PyBaseExceptionRef, ctx: C) -> anyhow::Error
where
    C: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    let mut v = Vec::new();
    rustpython_vm::exceptions::write_exception(&mut v, vm, &e).unwrap();
    let s = String::from_utf8(v).unwrap();
    anyhow::anyhow!("Python error:\n{}\n", s).context(ctx)
}

impl PickItUp {
    async fn new(opts: InitOptions, gfx: &Graphics) -> anyhow::Result<Self> {
        let InitOptions {
            filename,
            frozen,
            entry_module,
            ..
        } = opts;

        let mut path_list = Vec::new();
        let (source, code_path) = if cfg!(target_arch = "wasm32") {
            (None, "<qs>".to_owned())
        } else {
            // requires special handling because of complications in static folder of cargo-web
            let dir = std::env::current_dir().unwrap();
            let dir = if dir.ends_with("static") {
                Path::new("..")
            } else {
                &dir
            };

            let code_path = dir.join(filename.as_ref().unwrap());
            let parent_dir = code_path.parent().unwrap().to_str().unwrap();
            path_list.push(parent_dir.to_owned());
            let s = std::fs::read_to_string(&code_path)
                .with_context(|| format!("couldn't read file {}", code_path.display()))?;
            (Some(s), code_path.to_string_lossy().into_owned())
        };
        let settings = PySettings {
            path_list,
            ..Default::default()
        };
        let interp = Interpreter::new_with_init(settings, |vm| {
            vm.add_native_module(MOD_NAME.to_owned(), Box::new(pyqs::make_module));
            if let Some(frozen) = frozen {
                vm.add_frozen(frozen);
            }
            if cfg!(target_arch = "wasm32") {
                rustpython_vm::InitParameter::Internal
            } else {
                rustpython_vm::InitParameter::External
            }
        });
        let (state, sprites, update_fn, onload_fn, draw_fn, event_fn) =
            interp.enter(|vm| -> anyhow::Result<_> {
                let code = match source {
                    Some(source) => vm
                        .compile(&source, compile::Mode::Exec, code_path)
                        .context("Error parsing python code")?,
                    None => {
                        let code = vm
                            .state
                            .frozen
                            .get(entry_module.as_deref().unwrap())
                            .expect("no entry frozen module")
                            .code
                            .clone();
                        vm.ctx.new_code_object(code)
                    }
                };

                let scope = vm.new_scope_with_builtins();
                vm.run_code_obj(code, scope.clone())
                    .map_err(|e| handle_err(vm, e, "while initializing module"))?;

                let get_func = |name| {
                    scope
                        .globals
                        .get_item_option(name, vm)
                        .map_err(|e| handle_err(vm, e, "while initializing"))
                };

                let resource_cfg = Default::default();
                let state = match get_func("init")? {
                    Some(init_fn) => RESOURCES.set(&resource_cfg, || {
                        vm.invoke(&init_fn, vec![])
                            .map_err(|e| handle_err(vm, e, "in init function"))
                    })?,
                    None => vm.ctx.none(),
                };

                let update_fn = get_func("update")?;
                let onload_fn = get_func("onload")?;
                let draw_fn = get_func("draw")?;
                let event_fn = get_func("event")?;

                Ok((
                    state,
                    resource_cfg.into_inner(),
                    update_fn,
                    onload_fn,
                    draw_fn,
                    event_fn,
                ))
            })?;

        let sprites = Resources::new(sprites, gfx).await?.into();

        // create sprites based on resources

        Ok(PickItUp {
            interp,
            sprites,
            update_fn,
            draw_fn,
            event_fn,
            onload_fn,
            state,
            last_update: Instant::now(),
            window_initialized: false,
        })
    }

    fn set_context<R>(
        &self,
        gfx: &RefCell<Graphics>,
        state: &RefCell<State>,
        f: impl FnOnce() -> R,
    ) -> R {
        GRAPHICS.set(gfx, || STATE.set(state, || SPRITES.set(&self.sprites, f)))
    }

    fn event(&mut self, event: &Event, state: &mut RefCell<State>) -> anyhow::Result<()> {
        if let Some(event_fn) = &self.event_fn {
            self.interp.enter(|vm| -> anyhow::Result<()> {
                if let Some(evt) = event_to_py(vm, event, state.get_mut()) {
                    STATE.set(state, || {
                        vm.invoke(event_fn, vec![self.state.clone(), evt])
                            .map_err(|e| handle_err(vm, e, "in event function"))
                    })?;
                }
                Ok(())
            })?
        }

        Ok(())
    }

    fn update(
        &mut self,
        gfx: &mut RefCell<Graphics>,
        state: &mut RefCell<State>,
    ) -> anyhow::Result<()> {
        if !self.window_initialized {
            if let Some(onload_fn) = &self.onload_fn {
                self.set_context(gfx, state, || {
                    self.interp.enter(|vm| {
                        // invoke onload_fn
                        vm.invoke(onload_fn, vec![self.state.clone()])
                            .map(drop)
                            .map_err(|e| handle_err(vm, e, "in onload function"))
                    })
                })?
            }
            self.window_initialized = true;
        }

        let update_rate = state.get_mut().update_rate;
        let period = Duration::from_secs_f64(update_rate / 1000.0);
        if self.last_update.elapsed() >= period {
            self.last_update += period;
        } else {
            return Ok(());
        }

        // update animations
        self.sprites.get_mut().update_anim(update_rate);

        if let Some(update_fn) = &self.update_fn {
            self.set_context(gfx, state, || {
                self.interp.enter(|vm| {
                    vm.invoke(update_fn, vec![self.state.clone()])
                        .map(drop)
                        .map_err(|e| handle_err(vm, e, "in update function"))
                })
            })?;
        }
        Ok(())
    }

    fn draw(
        &mut self,
        gfx: &mut RefCell<Graphics>,
        state: &mut RefCell<State>,
    ) -> anyhow::Result<()> {
        gfx.get_mut().clear(Color::BLACK);

        if let Some(draw_fn) = &self.draw_fn {
            self.set_context(gfx, state, || {
                self.interp.enter(|vm| {
                    vm.invoke(draw_fn, vec![self.state.clone()])
                        .map(drop)
                        .map_err(|e| handle_err(vm, e, "in draw function"))
                })
            })?;
        }
        Ok(())
    }
}

fn event_to_py(vm: &VirtualMachine, event: &Event, state: &State) -> Option<PyObjectRef> {
    let d = vm.ctx.new_namespace();
    macro_rules! set {
        ($key:ident, $val:expr) => {
            vm.set_attr(&d, stringify!($key), IntoPyObject::into_pyobject($val, vm))
                .unwrap();
        };
    };
    match event {
        Event::FocusChanged(f) => {
            set!(
                event,
                if f.is_focused() {
                    "focused"
                } else {
                    "unfocused"
                }
            );
        }
        Event::KeyboardInput(k) => {
            let key = k.key();
            set!(event, "key");
            set!(key, format!("{:?}", key));
            set!(state, format!("{:?}", state.keyboard[key as usize]));
        }
        Event::ReceivedCharacter(c) => {
            set!(event, "typed");
            set!(char, c.character().to_string());
        }
        Event::PointerMoved(_) => {
            set!(event, "mouse_moved");
            set!(x, state.mouse_pos.x);
            set!(y, state.mouse_pos.y);
        }
        Event::PointerEntered(_) => {
            set!(event, "mouse_entered");
        }
        Event::PointerLeft(_) => {
            set!(event, "mouse_exited");
        }
        Event::ScrollInput(_) => {
            set!(event, "mouse_wheel");
            set!(x, state.wheel_delta.x);
            set!(y, state.wheel_delta.y);
        }
        Event::PointerInput(evt) => {
            let i = match evt.button() {
                MouseButton::Left => 0,
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
                MouseButton::Other(_) => return None,
            };
            set!(event, "mouse_button");
            set!(button, format!("{:?}", evt.button()));
            set!(down, evt.is_down());
            set!(state, format!("{:?}", state.mouse[i]));
        }
        // TODO: more events
        _ => return None,
    }
    Some(d)
}

pub struct InitOptions {
    pub width: i32,
    pub height: i32,
    pub filename: Option<PathBuf>,
    pub frozen: Option<HashMap<String, FrozenModule>>,
    pub entry_module: Option<String>,
}
impl Default for InitOptions {
    fn default() -> Self {
        InitOptions {
            width: 800,
            height: 600,
            filename: None,
            frozen: None,
            entry_module: None,
        }
    }
}

/// The current state of a button
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum ButtonState {
    /// The button was activated this frame
    Pressed = 0,
    /// The button is active but was not activated this frame
    Held = 1,
    /// The button was released this frame
    Released = 2,
    /// The button is not active but was not released this frame
    NotPressed = 3,
}

impl ButtonState {
    fn update(&mut self, new: bool) {
        let new = match (self.is_down(), new) {
            (false, false) => ButtonState::NotPressed,
            (false, true) => ButtonState::Pressed,
            (true, false) => ButtonState::Released,
            (true, true) => ButtonState::Held,
        };
        *self = new;
    }

    /// Determine if the button is either Pressed or Held
    fn is_down(&self) -> bool {
        match *self {
            ButtonState::Pressed | ButtonState::Held => true,
            ButtonState::Released | ButtonState::NotPressed => false,
        }
    }
}

pub struct State {
    update_rate: f64,
    keyboard: Box<[ButtonState; pyqs::NUM_KEYS]>,
    mouse: [ButtonState; 3],
    mouse_pos: Vector,
    wheel_delta: Vector,
    winsize: Vector,
}

impl State {
    fn process_event(&mut self, e: &Event, gfx: &Graphics, win: &Window) {
        match e {
            Event::KeyboardInput(k) => {
                self.keyboard[k.key() as usize].update(k.is_down());
            }
            Event::PointerEntered(_) => {}
            Event::PointerLeft(_) => {}
            Event::PointerMoved(p) => {
                self.mouse_pos = gfx.screen_to_camera(win, p.location());
            }
            Event::PointerInput(p) => {
                let i = match p.button() {
                    MouseButton::Left => 0,
                    MouseButton::Middle => 1,
                    MouseButton::Right => 2,
                    MouseButton::Other(_) => return,
                };
                self.mouse[i].update(p.is_down());
            }
            Event::ScrollInput(delta) => {
                use quicksilver::input::ScrollDelta;
                let v = match delta {
                    ScrollDelta::Lines(v) | ScrollDelta::Pixels(v) => v,
                };
                self.wheel_delta = Vector::new(v.x, v.y);
            }
            Event::Resized(r) => {
                self.winsize = r.size();
            }
            _ => {}
        }
    }
}

async fn app(
    opts: InitOptions,
    win: Window,
    gfx: Graphics,
    mut input: Input,
) -> anyhow::Result<()> {
    let mut pickitup = PickItUp::new(opts, &gfx).await?;
    let mut gfx = RefCell::new(gfx);
    let mut state = RefCell::new(State {
        update_rate: 1000.0 / 60.0,
        keyboard: Box::new([ButtonState::NotPressed; pyqs::NUM_KEYS]),
        mouse: [ButtonState::NotPressed; 3],
        mouse_pos: Vector::ZERO,
        wheel_delta: Vector::ZERO,
        winsize: win.size(),
    });

    loop {
        while let Some(e) = input.next_event().await {
            state.get_mut().process_event(&e, gfx.get_mut(), &win);
            pickitup.event(&e, &mut state)?;
        }

        pickitup.update(&mut gfx, &mut state)?;

        pickitup.draw(&mut gfx, &mut state)?;

        gfx.get_mut().present(&win)?;
    }
}

pub fn run(opts: InitOptions) -> ! {
    let size = Vector::new(opts.width as f32, opts.height as f32);
    let mut settings = quicksilver::Settings::default();
    settings.size = size;
    settings.title = "pickitup";
    quicksilver::run(settings, |w, gfx, input| app(opts, w, gfx, input))
}
