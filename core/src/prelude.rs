pub use std::cell::RefCell;

pub use num_traits::ToPrimitive;
pub use rustpython_vm::pyobject::{ItemProtocol, TypeProtocol};

pub use crate::anim::Animation;
pub use crate::resources::{ResourceConfig, Resources};
pub use quicksilver::{
    geom::{Circle, Line, Rectangle, Transform, Triangle, Vector},
    graphics::{Color, Image},
    Timer, // lifecycle::{run, Asset, Settings, State, Window},
};

pub use futures::prelude::*;

pub use quicksilver::{
    // combinators::{join_all, result},
    geom::Shape,
    graphics::{FontRenderer, Graphics, VectorFont},
    // graphics::{Background::Img, Drawable, Font, FontStyle},
    input::{Event, Key, MouseButton},
    // lifecycle::Event,
    load_file,
    saving::*,
    // sound::Sound,
    // Error,
    // Future,
    Input,
    QuicksilverError,
    Result as QsResult,
    Window,
};

pub use std::future::Future;

pub use rustpython_vm::{
    builtins::{PyDictRef, PyFloat, PyInt, PyStrRef},
    common::borrow::BorrowValue,
    common::rc::PyRc,
    compile,
    exceptions::PyBaseExceptionRef,
    function::{FuncArgs, OptionalArg},
    pyobject::{IntoPyObject, PyContext, PyObjectRef, PyResult, TryFromObject},
    stdlib::StdlibInitFunc,
    Interpreter, VirtualMachine,
};

pub const MOD_NAME: &'static str = "qs";

use scoped_tls::scoped_thread_local;

scoped_thread_local!(pub static SPRITES: RefCell<Resources>);
scoped_thread_local!(pub static RESOURCES: RefCell<ResourceConfig>);
scoped_thread_local!(pub static GRAPHICS: RefCell<Graphics>);
scoped_thread_local!(pub static STATE: RefCell<crate::State>);
