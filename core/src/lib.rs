#[macro_use]
extern crate rustpython_vm;

mod anim;
mod prelude;
mod pyqs;
mod resources;

use rustpython_vm::{bytecode::FrozenModule, PySettings};
use scoped_tls::scoped_thread_local;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::prelude::*;

scoped_thread_local!(pub static FNAME: Cell<PathBuf>);
scoped_thread_local!(pub static FROZEN: Cell<HashMap<String, FrozenModule>>);

struct PickItUp {
    interp: Interpreter,
    sprites: RefCell<Asset<Resources>>,

    update_fn: Option<PyObjectRef>,
    draw_fn: Option<PyObjectRef>,
    onload_fn: Option<PyObjectRef>,
    event_fn: Option<PyObjectRef>,
    state: PyObjectRef,

    window_initialized: bool,
}

fn handle_err(vm: &VirtualMachine, e: PyBaseExceptionRef, ctx: &str) -> Error {
    let s = vm.to_str(e.as_object());
    let s = s
        .as_ref()
        .map_or("Error, and error getting error message", |s| {
            s.borrow_value()
        });
    Error::ContextError(format!("Error {}: {}", ctx, s))
}

impl PickItUp {
    fn with_window_ptr<R>(&self, window: &mut Window, f: impl FnOnce() -> R) -> R {
        WindowHandle::set(window, || SPRITES.set(&self.sprites, f))
    }
}

impl State for PickItUp {
    fn new() -> Result<Self> {
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

            let fname = FNAME.with(|fname| fname.take());
            let code_path = dir.join(&fname);
            let parent_dir = code_path.parent().unwrap().to_str().unwrap();
            path_list.push(parent_dir.to_owned());
            let s = std::fs::read_to_string(&code_path)
                .unwrap_or_else(|_| panic!("couldn't read file {}", code_path.display()));
            (Some(s), code_path.to_string_lossy().into_owned())
        };
        let settings = PySettings {
            path_list,
            ..Default::default()
        };
        let interp = Interpreter::new_with_init(settings, |vm| {
            let state = PyRc::get_mut(&mut vm.state).unwrap();
            state
                .stdlib_inits
                .insert(MOD_NAME.to_owned(), Box::new(pyqs::make_module));
            if FROZEN.is_set() {
                let frozen = FROZEN.with(|f| f.take());
                state.frozen.extend(frozen);
            }
            if cfg!(target_arch = "wasm32") {
                rustpython_vm::InitParameter::Internal
            } else {
                rustpython_vm::InitParameter::External
            }
        });
        let (state, sprites, update_fn, onload_fn, draw_fn, event_fn) =
            interp.enter(|vm| -> Result<_> {
                let code = match source {
                    Some(source) => vm
                        .compile(&source, compile::Mode::Exec, code_path)
                        .map_err(|err| {
                            Error::ContextError(format!("Error parsing Python code: {}", err))
                        })?,
                    None => {
                        let code = vm
                            .state
                            .frozen
                            .get("run")
                            .expect("no 'run' frozen module")
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
                // create sprites based on resources
                let sprites = Asset::new(Resources::new(resource_cfg.into_inner())).into();

                let update_fn = get_func("update")?;
                let onload_fn = get_func("onload")?;
                let draw_fn = get_func("draw")?;
                let event_fn = get_func("event")?;

                Ok((state, sprites, update_fn, onload_fn, draw_fn, event_fn))
            })?;

        Ok(PickItUp {
            interp,
            sprites,
            update_fn,
            draw_fn,
            event_fn,
            onload_fn,
            state,
            window_initialized: false,
        })
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        if let Some(event_fn) = &self.event_fn {
            self.with_window_ptr(window, || {
                self.interp.enter(|vm| -> Result<()> {
                    if let Some(evt) = event_to_py(vm, event) {
                        vm.invoke(event_fn, vec![self.state.clone(), evt])
                            .map_err(|e| handle_err(vm, e, "in event function"))?;
                    }
                    Ok(())
                })
            })?
        }

        Ok(())
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        if !self.window_initialized {
            if let Some(onload_fn) = &self.onload_fn {
                self.with_window_ptr(window, || {
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

        // update animations
        self.sprites.get_mut().execute(|spr| {
            spr.update_anim(window)?;
            Ok(())
        })?;

        if let Some(update_fn) = &self.update_fn {
            self.with_window_ptr(window, || {
                self.interp.enter(|vm| {
                    vm.invoke(update_fn, vec![self.state.clone()])
                        .map(drop)
                        .map_err(|e| handle_err(vm, e, "in update function"))
                })
            })?;
        }
        Ok(())
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;

        if let Some(draw_fn) = &self.draw_fn {
            self.with_window_ptr(window, || {
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

fn event_to_py(vm: &VirtualMachine, event: &Event) -> Option<PyObjectRef> {
    let d = vm.ctx.new_namespace();
    macro_rules! set {
        ($key:ident, $val:expr) => {
            vm.set_attr(&d, stringify!($key), IntoPyObject::into_pyobject($val, vm))
                .unwrap();
        };
    };
    match event {
        Event::Closed => {
            set!(event, "closed");
        }
        Event::Focused => {
            set!(event, "focused");
        }
        Event::Unfocused => {
            set!(event, "unfocused");
        }
        Event::Key(key, state) => {
            set!(event, "key");
            set!(key, format!("{:?}", key));
            set!(state, format!("{:?}", state));
        }
        Event::Typed(c) => {
            set!(event, "typed");
            set!(char, format!("{:?}", c));
        }
        Event::MouseMoved(v) => {
            set!(event, "mouse_moved");
            set!(x, v.x);
            set!(y, v.y);
        }
        Event::MouseEntered => {
            set!(event, "mouse_entered");
        }
        Event::MouseExited => {
            set!(event, "mouse_exited");
        }
        Event::MouseWheel(v) => {
            set!(event, "mouse_wheel");
            set!(x, v.x);
            set!(y, v.y);
        }
        Event::MouseButton(button, state) => {
            set!(event, "mouse_button");
            set!(button, format!("{:?}", button));
            set!(state, format!("{:?}", state));
        }
        // Event::GamepadAxis(i32, GamepadAxis, f32),
        // Event::GamepadButton(i32, GamepadButton, ButtonState),
        // Event::GamepadConnected(i32),
        // Event::GamepadDisconnected(i32)
        // TODO: more events
        _ => return None,
    }
    Some(d)
}

pub fn run(w: i32, h: i32) {
    quicksilver::prelude::run::<PickItUp>("pickitup", Vector::new(w, h), Settings::default());
}
