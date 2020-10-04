use crate::prelude::*;

use rustpython_vm::function::FromArgs;
use rustpython_vm::obj::{objfloat, objint};
use rustpython_vm::pyobject::PyIterable;

pub(crate) use qs::make_module;

macro_rules! extract_list {
    ($vm:expr, $obj:expr, $err:literal, $t:ty, $n:expr) => {
        $vm.extract_elements::<$t>(&$obj).and_then(|v| {
            <Box<[$t; $n]> as std::convert::TryFrom<_>>::try_from(v.into_boxed_slice())
                .map(|b| *b)
                .map_err(|_| $vm.new_type_error($err.to_owned()))
        })
    };
    ($vm:expr, $obj:expr, $err:literal, ($(($i:ident, $t:ty)),*)) => {
        extract_list!($vm, $obj, $err, PyObjectRef, extract_list!(@count($($i)*)))
            .and_then(|[$($i),*]| {
                Ok((
                    $(<$t>::try_from_object($vm, $i)?),*
                ))
            })
    };
    (@count($($t:tt)*)) => {{
        0 $(+ extract_list!(@nop($t)))*
    }};
    (@nop($t:tt)) => (1);
}

struct TransformDim([f32; 3]);
impl TryFromObject for TransformDim {
    fn try_from_object(vm: &VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let [x, y, z] = extract_list!(
            vm,
            obj,
            "expected transform matrix dimension to be a 3-tuple",
            PyNum,
            3
        )?;
        Ok(Self([x.to_f32(), y.to_f32(), z.to_f32()]))
    }
}
struct PyTransform(Transform);
impl TryFromObject for PyTransform {
    fn try_from_object(vm: &VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let [a, b, c] = extract_list!(
            vm,
            obj,
            "expected transform matrix to have 3 dimensions",
            TransformDim,
            3
        )?;
        Ok(Self(Transform::from_array([a.0, b.0, c.0])))
    }
}

#[derive(Copy, Clone)]
struct Point(f32, f32);
impl From<Point> for Vector {
    fn from(p: Point) -> Self {
        Vector::new(p.0, p.1)
    }
}
impl TryFromObject for Point {
    fn try_from_object(vm: &VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let [x, y] = extract_list!(vm, obj, "expected only 2 elements for a point", PyNum, 2)?;
        Ok(Point(x.to_f32(), y.to_f32()))
    }
}

/// [[x1, y1], [x2, y2], [x3, y3]]
struct PyTriangle(Triangle);
impl TryFromObject for PyTriangle {
    fn try_from_object(vm: &VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let [p0, p1, p2] = extract_list!(vm, obj, "expected 3 points for a triangle", Point, 3)?;
        Ok(Self(Triangle::new(p0, p1, p2)))
    }
}

/// [[x1, y1], [x2, y2]]
struct PyRect(Rectangle);
impl TryFromObject for PyRect {
    fn try_from_object(vm: &VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let [p0, p1] = extract_list!(vm, obj, "expected 2 points for a rectangle", Point, 2)?;
        Ok(Self(Rectangle::new(p0, p1)))
    }
}

struct PyColor(Color);
impl TryFromObject for PyColor {
    fn try_from_object(vm: &VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let [r, g, b, a] = extract_list!(vm, obj, "expected a 4-vector for a color", f32, 4)?;
        Ok(Self(Color { r, g, b, a }))
    }
}

struct PyNum(f64);
impl TryFromObject for PyNum {
    fn try_from_object(vm: &VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let f = match_class!(match obj {
            i @ objint::PyInt => i
                .borrow_value()
                .to_f64()
                .ok_or_else(|| vm.new_type_error("int can't fit into f32".to_owned()))?,
            f @ objfloat::PyFloat => f.to_f64(),
            _ => return Err(vm.new_type_error("expected a number".to_owned())),
        });
        Ok(PyNum(f))
    }
}
impl PyNum {
    fn to_f32(self) -> f32 {
        self.0 as f32
    }
}

#[pymodule]
mod qs {
    use super::*;

    // INITIALIZATION FUNCTIONS

    #[pyfunction]
    fn init_sprites(imgs: PyIterable, vm: &VirtualMachine) -> PyResult<()> {
        let it = imgs.iter(vm)?.map(|el| -> PyResult<_> {
            let [name, path] = extract_list!(
                vm,
                el?,
                "expected [name, path] for init_sprites",
                PyStrRef,
                2
            )?;
            Ok((
                name.borrow_value().to_owned(),
                path.borrow_value().to_owned(),
            ))
        });
        itertools::process_results(it, |it| {
            RESOURCES.with(|r| {
                r.borrow_mut().imgs.extend(it);
            })
        })
    }

    #[pyfunction]
    fn init_anims(anims: PyIterable, vm: &VirtualMachine) -> PyResult<()> {
        let it = anims.iter(vm)?.map(|el| -> PyResult<_> {
            let (name, path, nframes, dur) = extract_list!(
                vm,
                el?,
                "expected [name, path] for init_anims",
                ((a, PyStrRef), (b, PyStrRef), (c, usize), (d, f64))
            )?;
            Ok((
                name.borrow_value().to_owned(),
                path.borrow_value().to_owned(),
                (nframes, dur),
            ))
        });
        itertools::process_results(it, |it| {
            RESOURCES.with(|r| {
                r.borrow_mut().anims.extend(it);
            })
        })
    }

    #[pyfunction]
    fn init_sounds(sounds: PyIterable, vm: &VirtualMachine) -> PyResult<()> {
        let it = sounds.iter(vm)?.map(|el| -> PyResult<_> {
            let [name, path] = extract_list!(
                vm,
                el?,
                "expected [name, path] for init_sounds",
                PyStrRef,
                2
            )?;
            Ok((
                name.borrow_value().to_owned(),
                path.borrow_value().to_owned(),
            ))
        });
        itertools::process_results(it, |it| {
            RESOURCES.with(|r| {
                r.borrow_mut().sounds.extend(it);
            })
        })
    }

    #[pyfunction]
    fn init_fonts(fonts: PyIterable, vm: &VirtualMachine) -> PyResult<()> {
        let it = fonts.iter(vm)?.map(|el| -> PyResult<_> {
            let [name, path] =
                extract_list!(vm, el?, "expected [name, path] for init_fonts", PyStrRef, 2)?;
            Ok((
                name.borrow_value().to_owned(),
                path.borrow_value().to_owned(),
            ))
        });
        itertools::process_results(it, |it| {
            RESOURCES.with(|r| {
                r.borrow_mut().fonts.extend(it);
            })
        })
    }

    // WINDOW FUNCTIONS

    #[pyfunction]
    fn clear(PyColor(color): PyColor, vm: &VirtualMachine) -> PyResult<()> {
        WINDOW.with(|w| {
            w.borrow_mut()
                .clear(color)
                .map_err(|e| vm.new_runtime_error(e.to_string()))
        })
    }

    // TODO: namedtuple?
    fn new_py_point(vm: &VirtualMachine, v: Vector) -> PyObjectRef {
        vm.ctx.new_tuple(vec![
            vm.ctx.new_float(v.x.into()),
            vm.ctx.new_float(v.y.into()),
        ])
    }

    #[pyfunction]
    fn mouse_wheel_delta(vm: &VirtualMachine) -> PyObjectRef {
        let v = WINDOW.with(|w| w.borrow().mouse().wheel());
        new_py_point(vm, v)
    }

    #[pyfunction]
    fn mouse_pos(vm: &VirtualMachine) -> PyObjectRef {
        let v = WINDOW.with(|w| w.borrow().mouse().pos());
        new_py_point(vm, v)
    }

    #[pyfunction]
    fn update_rate() -> f64 {
        WINDOW.with(|w| w.borrow().update_rate())
    }

    #[pyfunction]
    fn set_update_rate(PyNum(rate): PyNum) {
        WINDOW.with(|w| w.borrow_mut().set_update_rate(rate))
    }

    #[pyfunction]
    fn keyboard(vm: &VirtualMachine) -> PyResult<PyDictRef> {
        WINDOW.with(|w| {
            let window = w.borrow();
            let keys = window.keyboard();
            let d = vm.ctx.new_dict();
            for key in KEY_LIST {
                let val = vm.ctx.new_str(format!("{:?}", keys[*key]));
                d.set_item(&*format!("{:?}", key), val, vm)?;
            }
            Ok(d)
        })
    }

    #[pyfunction]
    fn keyboard_bool(vm: &VirtualMachine) -> PyResult<PyDictRef> {
        WINDOW.with(|w| {
            let window = w.borrow();
            let keys = window.keyboard();
            let d = vm.ctx.new_dict();
            for key in KEY_LIST {
                let val = vm.ctx.new_bool(keys[*key].is_down());
                d.set_item(&*format!("{:?}", key), val, vm)?;
            }
            Ok(d)
        })
    }

    #[pyfunction]
    fn set_view(PyRect(rect): PyRect) {
        let view = quicksilver::graphics::View::new(rect);
        WINDOW.with(|w| w.borrow_mut().set_view(view))
    }

    // SHAPE FUNCTIONS

    #[derive(FromArgs)]
    struct ShapeArgs {
        #[pyarg(named, default = "PyTransform(Transform::IDENTITY)")]
        transform: PyTransform,
        #[pyarg(named, default = "0")]
        z: i32,
        #[pyarg(named, default = "PyColor(Color::RED)")]
        color: PyColor,
    }
    impl ShapeArgs {
        fn into_drawable(self) -> (Background<'static>, Transform, i32) {
            (Background::Col(self.color.0), self.transform.0, self.z)
        }
    }

    #[pyfunction]
    fn rect(PyRect(rect): PyRect, args: ShapeArgs) {
        let (bkg, trans, z) = args.into_drawable();
        WINDOW.with(|w| w.borrow_mut().draw_ex(&rect, bkg, trans, z))
    }

    #[pyfunction]
    fn circ(center: Point, radius: PyNum, args: ShapeArgs) {
        let (bkg, trans, z) = args.into_drawable();
        let circle = Circle::new(center, radius.to_f32());
        WINDOW.with(|w| w.borrow_mut().draw_ex(&circle, bkg, trans, z))
    }

    #[pyfunction]
    fn triangle(PyTriangle(tri): PyTriangle, args: ShapeArgs) {
        let (bkg, trans, z) = args.into_drawable();
        WINDOW.with(|w| w.borrow_mut().draw_ex(&tri, bkg, trans, z))
    }

    #[derive(FromArgs)]
    struct LineArgs {
        #[pyarg(named, default = "PyNum(1.0)")]
        thickness: PyNum,
        #[pyarg(flatten)]
        common: ShapeArgs,
    }

    #[pyfunction]
    fn line(PyRect(rect): PyRect, args: LineArgs) {
        let line = Line::new(rect.pos, rect.size).with_thickness(args.thickness.to_f32());
        let (bkg, trans, z) = args.common.into_drawable();
        WINDOW.with(|w| w.borrow_mut().draw_ex(&line, bkg, trans, z))
    }

    // DRAW FUNCTIONS

    #[pyfunction]
    fn sound(s: PyStrRef, vm: &VirtualMachine) -> PyResult<()> {
        let mut exists = true;
        SPRITES.with(|r| {
            r.borrow_mut()
                .execute(|r| match r.get_sound(s.borrow_value()) {
                    Some(sound) => sound.play(),
                    None => {
                        exists = false;
                        Ok(())
                    }
                })
                .map_err(|e| vm.new_runtime_error(e.to_string()))
        })?;
        if exists {
            Ok(())
        } else {
            Err(vm.new_lookup_error(format!("sound {:?} does not exist", s.borrow_value())))
        }
    }

    #[derive(FromArgs)]
    struct CommonArgs {
        #[pyarg(named, default = "PyTransform(Transform::IDENTITY)")]
        transform: PyTransform,
        #[pyarg(named, default = "0")]
        z: i32,
    }
    impl CommonArgs {
        fn into_drawable(self) -> (Transform, i32) {
            (self.transform.0, self.z)
        }
    }

    enum RectOrPoint {
        Rect(Rectangle),
        Point(Point),
    }
    impl RectOrPoint {
        fn rect_from_img(self, im: &Image) -> Rectangle {
            match self {
                RectOrPoint::Rect(rect) => rect,
                RectOrPoint::Point(p0) => Rectangle::new(p0, im.area().size),
            }
        }
    }
    impl FromArgs for RectOrPoint {
        fn from_args(
            vm: &VirtualMachine,
            args: &mut PyFuncArgs,
        ) -> std::result::Result<Self, rustpython_vm::function::ArgumentError> {
            match (args.take_keyword("rect"), args.take_keyword("p0")) {
                (Some(rect), None) => Ok(Self::Rect(PyRect::try_from_object(vm, rect)?.0)),
                (None, Some(point)) => Ok(Self::Point(Point::try_from_object(vm, point)?)),
                (None, None) => Err("sprite() must have either `p0=` or `rect=` named argument"),
                (Some(_), Some(_)) => {
                    Err("sprite() must have either `p0=` or `rect=` named argument, but not both")
                }
            }
            .map_err(|s| vm.new_type_error(s.to_owned()).into())
        }

        fn arity() -> std::ops::RangeInclusive<usize> {
            0..=1
        }
    }

    #[derive(FromArgs)]
    struct SpriteArgs {
        #[pyarg(flatten)]
        position: RectOrPoint,
        #[pyarg(flatten)]
        common: CommonArgs,
    }

    #[pyfunction]
    fn sprite(name: PyStrRef, args: SpriteArgs, vm: &VirtualMachine) -> PyResult<()> {
        let name = name.borrow_value();
        let position = args.position;
        let (trans, z) = args.common.into_drawable();
        WINDOW.with(|w| {
            SPRITES.with(|r| {
                let mut exists = true;
                let res = r.borrow_mut().execute(|resources| {
                    match resources.get_img(&name) {
                        Some(im) => {
                            w.borrow_mut()
                                .draw_ex(&position.rect_from_img(&im), Img(im), trans, z)
                        }
                        None => exists = false,
                    }
                    Ok(())
                });
                let () = res.map_err(|e| vm.new_runtime_error(e.to_string()))?;
                if exists {
                    Ok(())
                } else {
                    Err(vm.new_lookup_error(format!("sprite {:?} does not exist", name)))
                }
            })
        })
    }

    #[derive(FromArgs)]
    struct TextArgs {
        #[pyarg(named, default = "PyNum(90.0)")]
        pt: PyNum,
        #[pyarg(named, default = "PyColor(Color::BLACK)")]
        color: PyColor,
        #[pyarg(named, optional)]
        font: OptionalArg<PyStrRef>,
        #[pyarg(named, default = "true")]
        store_in_cache: bool,
        #[pyarg(flatten)]
        position: RectOrPoint,
        #[pyarg(flatten)]
        common: CommonArgs,
    }

    #[pyfunction]
    fn text(text: PyStrRef, args: TextArgs, vm: &VirtualMachine) -> PyResult<()> {
        let (trans, z) = args.common.into_drawable();
        let font_name = args.font.as_option().map(|s| s.borrow_value());
        let text = text.borrow_value();
        let TextArgs {
            position,
            store_in_cache,
            ..
        } = args;

        let style = FontStyle::new(args.pt.to_f32(), args.color.0);

        let mut exists = false;
        let res = WINDOW.with(|w| {
            SPRITES.with(|r| {
                r.borrow_mut().execute(|resources| {
                    let im = resources.render_str(font_name, text, style, store_in_cache)?;
                    match im {
                        Some(im) => {
                            w.borrow_mut()
                                .draw_ex(&position.rect_from_img(&im), Img(&im), trans, z)
                        }
                        _ => exists = false,
                    }
                    Ok(())
                })
            })
        });
        let () = res.map_err(|e| vm.new_runtime_error(e.to_string()))?;

        if exists {
            Ok(())
        } else {
            Err(vm.new_lookup_error(format!("font {:?} does not exist", font_name.unwrap_or(""))))
        }
    }

    #[derive(FromArgs)]
    struct AnimArgs {
        #[pyarg(flatten)]
        position: RectOrPoint,
        #[pyarg(flatten)]
        common: CommonArgs,
    }

    #[pyfunction]
    fn anim(name: PyStrRef, args: AnimArgs, vm: &VirtualMachine) -> PyResult<()> {
        let name = name.borrow_value();
        let position = args.position;
        let (trans, z) = args.common.into_drawable();

        let mut exists = true;
        let res = WINDOW.with(|w| {
            SPRITES.with(|r| {
                r.borrow_mut().execute(|resources| {
                    match resources.get_anim(name) {
                        Some(a) => {
                            let im = a.current_frame();
                            w.borrow_mut()
                                .draw_ex(&position.rect_from_img(im), Img(im), trans, z)
                        }
                        None => exists = false,
                    }
                    Ok(())
                })
            })
        });
        let () = res.map_err(|e| vm.new_runtime_error(e.to_string()))?;
        if exists {
            Ok(())
        } else {
            Err(vm.new_lookup_error(format!("animation {:?} does not exist", name)))
        }
    }

    #[pyfunction]
    fn set_anim_duration(name: PyStrRef, dur: f64, vm: &VirtualMachine) -> PyResult<()> {
        let name = name.borrow_value();

        let mut exists = false;
        let res = SPRITES.with(|r| {
            r.borrow_mut().execute(|resources| {
                match resources.get_anim_mut(name) {
                    Some(a) => a.set_duration(dur),
                    None => exists = false,
                }
                Ok(())
            })
        });
        let () = res.map_err(|e| vm.new_runtime_error(e.to_string()))?;
        if exists {
            Ok(())
        } else {
            Err(vm.new_lookup_error(format!("animation {:?} does not exist", name)))
        }
    }
}

// has to be outside of the pymodule or the attribute macro hangs forever :/
#[rustfmt::skip]
const KEY_LIST: &[Key] = &[Key::Key1, Key::Key2, Key::Key3, Key::Key4, Key::Key5, Key::Key6,
    Key::Key7, Key::Key8, Key::Key9, Key::Key0, Key::A, Key::B, Key::C, Key::D, Key::E, Key::F,
    Key::G, Key::H, Key::I, Key::J, Key::K, Key::L, Key::M, Key::N, Key::O, Key::P, Key::Q,
    Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X, Key::Y, Key::Z, Key::Escape, Key::F1,
    Key::F2, Key::F3, Key::F4, Key::F5, Key::F6, Key::F7, Key::F8, Key::F9, Key::F10, Key::F11,
    Key::F12, Key::F13, Key::F14, Key::F15, Key::F16, Key::F17, Key::F18, Key::F19, Key::F20,
    Key::F21, Key::F22, Key::F23, Key::F24, Key::Snapshot, Key::Scroll, Key::Pause, Key::Insert,
    Key::Home, Key::Delete, Key::End, Key::PageDown, Key::PageUp, Key::Left, Key::Up, Key::Right,
    Key::Down, Key::Back, Key::Return, Key::Space, Key::Compose, Key::Caret, Key::Numlock,
    Key::Numpad0, Key::Numpad1, Key::Numpad2, Key::Numpad3, Key::Numpad4, Key::Numpad5,
    Key::Numpad6, Key::Numpad7, Key::Numpad8, Key::Numpad9, Key::AbntC1, Key::AbntC2, Key::Add,
    Key::Apostrophe, Key::Apps, Key::At, Key::Ax, Key::Backslash, Key::Calculator, Key::Capital,
    Key::Colon, Key::Comma, Key::Convert, Key::Decimal, Key::Divide, Key::Equals, Key::Grave,
    Key::Kana, Key::Kanji, Key::LAlt, Key::LBracket, Key::LControl, Key::LShift, Key::LWin,
    Key::Mail, Key::MediaSelect, Key::MediaStop, Key::Minus, Key::Multiply, Key::Mute,
    Key::MyComputer, Key::NavigateForward, Key::NavigateBackward, Key::NextTrack, Key::NoConvert,
    Key::NumpadComma, Key::NumpadEnter, Key::NumpadEquals, Key::OEM102, Key::Period, Key::PlayPause,
    Key::Power, Key::PrevTrack, Key::RAlt, Key::RBracket, Key::RControl, Key::RShift, Key::RWin,
    Key::Semicolon, Key::Slash, Key::Sleep, Key::Stop, Key::Subtract, Key::Sysrq, Key::Tab,
    Key::Underline, Key::Unlabeled, Key::VolumeDown, Key::VolumeUp, Key::Wake, Key::WebBack,
    Key::WebFavorites, Key::WebForward, Key::WebHome, Key::WebRefresh, Key::WebSearch, Key::WebStop,
    Key::Yen
];
