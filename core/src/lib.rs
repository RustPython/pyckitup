#[macro_use]
extern crate rustpython_vm;

mod anim;
mod prelude;
mod pyqs;
mod resources;

use once_cell::sync::OnceCell;
use rustpython_vm::PySettings;
use std::path::Path;

pub static FNAME: OnceCell<String> = OnceCell::new();

use crate::prelude::*;

struct PickItUp {
    interp: Interpreter,
    sprites: Option<RefCell<Asset<Resources>>>,

    update_fn: Option<PyObjectRef>,
    draw_fn: Option<PyObjectRef>,
    onload_fn: Option<PyObjectRef>,
    event_fn: Option<PyObjectRef>,
    state: Option<PyObjectRef>,

    window_initialized: bool,

    resource_cfg: RefCell<ResourceConfig>,
    code_loaded: bool,
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
    fn load_code(&mut self, source: &str, code_path: String) -> Result<()> {
        let Self {
            interp,
            sprites,
            update_fn,
            draw_fn,
            onload_fn,
            event_fn,
            state,
            resource_cfg,
            code_loaded,
            ..
        } = self;

        interp.enter(|vm| {
            let code = vm
                .compile(&source, compile::Mode::Exec, code_path)
                .map_err(|err| {
                    Error::ContextError(format!("Error parsing Python code: {}", err))
                })?;

            let scope = vm.new_scope_with_builtins();
            vm.run_code_obj(code, scope.clone())
                .map_err(|e| handle_err(vm, e, "while initializing module"))?;

            let get_func = |name| {
                scope
                    .globals
                    .get_item_option(name, vm)
                    .map_err(|e| handle_err(vm, e, "while initializing"))
            };

            *state = Some(match get_func("init")? {
                Some(init_fn) => RESOURCES.set(resource_cfg, || {
                    vm.invoke(&init_fn, vec![])
                        .map_err(|e| handle_err(vm, e, "in init function"))
                })?,
                None => vm.ctx.none(),
            });
            // create sprites based on resources
            *sprites = Some(Asset::new(Resources::new(resource_cfg.get_mut().clone())).into());

            *update_fn = get_func("update")?;
            *onload_fn = get_func("onload")?;
            *draw_fn = get_func("draw")?;
            *event_fn = get_func("event")?;

            *code_loaded = true;

            Ok(())
        })
    }

    fn with_window_ptr<R>(&self, window: &mut Window, f: impl FnOnce() -> R) -> R {
        WindowHandle::set(window, || {
            if let Some(r) = &self.sprites {
                SPRITES.set(r, f)
            } else {
                f()
            }
        })
    }
}

impl State for PickItUp {
    fn new() -> Result<Self> {
        let mut path_list = Vec::new();
        let (source, code_path) = if cfg!(target_arch = "wasm32") {
            (
                String::from_utf8(load_raw("test", "run.py")?).unwrap(),
                "<qs>".to_owned(),
            )
        } else {
            // requires special handling because of complications in static folder of cargo-web
            let dir = std::env::current_dir().unwrap();
            let dir = if dir.ends_with("static") {
                ".."
            } else {
                dir.to_str().unwrap()
            };

            let fname = FNAME.get().unwrap();
            let code_path = format!("{}/{}", dir, fname);
            let parent_dir = Path::new(&code_path).parent().unwrap().to_str().unwrap();
            path_list.push(parent_dir.to_owned());
            let s = std::fs::read_to_string(&code_path)
                .unwrap_or_else(|_| panic!("couldn't read file {}", code_path));
            (s, code_path)
        };
        let settings = PySettings {
            path_list,
            ..Default::default()
        };
        let interp = Interpreter::new_with_init(settings, |vm| {
            Rc::get_mut(&mut vm.state)
                .unwrap()
                .stdlib_inits
                .insert(MOD_NAME.to_owned(), Box::new(pyqs::make_module));
            rustpython_vm::InitParameter::External
        });
        let resources = None;
        let resource_cfg = Default::default();
        let mut ret = PickItUp {
            interp,
            sprites: resources,
            update_fn: None,
            draw_fn: None,
            event_fn: None,
            onload_fn: None,
            state: None,
            resource_cfg,
            code_loaded: false,
            window_initialized: false,
        };
        ret.load_code(&source, code_path)?;
        Ok(ret)
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        if let (Some(event_fn), Some(state)) = (&self.event_fn, &self.state) {
            self.with_window_ptr(window, || {
                self.interp.enter(|vm| -> Result<()> {
                    if let Some(evt) = event_to_py(vm, event) {
                        vm.invoke(event_fn, vec![state.clone(), evt])
                            .map_err(|e| handle_err(vm, e, "in event function"))?;
                    }
                    Ok(())
                })
            })?
        }

        Ok(())
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        if !self.code_loaded {
            return Ok(());
        }

        if !self.window_initialized {
            if let (Some(onload_fn), Some(state)) = (&self.onload_fn, &self.state) {
                RESOURCES.set(&self.resource_cfg, || {
                    self.with_window_ptr(window, || {
                        self.interp.enter(|vm| {
                            // invoke onload_fn
                            vm.invoke(onload_fn, vec![state.clone()])
                                .map(drop)
                                .map_err(|e| handle_err(vm, e, "in onload function"))
                        })
                    })
                })?
            }
            self.window_initialized = true;
        }

        // update animations
        if let Some(ref mut sprites) = &mut self.sprites {
            sprites.get_mut().execute(|spr| {
                spr.update_anim(window)?;
                Ok(())
            })?;
        }

        if let (Some(update_fn), Some(state)) = (&self.update_fn, &self.state) {
            self.with_window_ptr(window, || {
                self.interp.enter(|vm| {
                    vm.invoke(update_fn, vec![state.clone()])
                        .map(drop)
                        .map_err(|e| handle_err(vm, e, "in update function"))
                })
            })?;
        }
        Ok(())
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;
        if !self.code_loaded {
            return Ok(());
        }

        if let (Some(draw_fn), Some(state)) = (&self.draw_fn, &self.state) {
            self.with_window_ptr(window, || {
                self.interp.enter(|vm| {
                    vm.invoke(draw_fn, vec![state.clone()])
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
