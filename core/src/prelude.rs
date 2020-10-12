pub use std::cell::RefCell;

pub use num_traits::ToPrimitive;
pub use rustpython_vm::pyobject::{ItemProtocol, TypeProtocol};

pub use crate::anim::Animation;
pub use crate::resources::{ResourceConfig, Resources};
pub use quicksilver::{
    geom::{Circle, Line, Rectangle, Transform, Triangle, Vector},
    graphics::{Background, Color, Image},
    lifecycle::{run, Asset, Settings, State, Window},
};

pub use quicksilver::{
    combinators::{join_all, result},
    geom::Shape,
    graphics::{Background::Img, Drawable, Font, FontStyle},
    input::{ButtonState, Key, MouseButton},
    lifecycle::Event,
    load_file,
    saving::*,
    sound::Sound,
    Error, Future, Result,
};

pub use rustpython_compiler::compile;
pub use rustpython_vm::{
    builtins::{PyDictRef, PyFloat, PyInt, PyStrRef},
    common::borrow::BorrowValue,
    common::rc::PyRc,
    exceptions::PyBaseExceptionRef,
    function::{FuncArgs, OptionalArg},
    pyobject::{IntoPyObject, PyContext, PyObjectRef, PyResult, TryFromObject},
    stdlib::StdlibInitFunc,
    Interpreter, VirtualMachine,
};

pub const MOD_NAME: &'static str = "qs";

use scoped_tls::scoped_thread_local;

scoped_thread_local!(pub static SPRITES: RefCell<Asset<Resources>>);
scoped_thread_local!(pub static RESOURCES: RefCell<ResourceConfig>);
scoped_thread_local!(pub static WINDOW: RefCell<WindowHandle>);
pub struct WindowHandle {
    x: std::ptr::NonNull<Window>,
}
use std::ops::{Deref, DerefMut};
impl Deref for WindowHandle {
    type Target = Window;
    fn deref(&self) -> &Window {
        unsafe { self.x.as_ref() }
    }
}
impl DerefMut for WindowHandle {
    fn deref_mut(&mut self) -> &mut Window {
        unsafe { self.x.as_mut() }
    }
}
impl WindowHandle {
    pub fn set<R>(x: &mut Window, f: impl FnOnce() -> R) -> R {
        let x = x.into();
        let handle = RefCell::new(Self { x });
        WINDOW.set(&handle, f)
    }
}
