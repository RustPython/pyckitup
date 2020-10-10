use crate::prelude::*;
use std::borrow::Cow;
use std::collections::{hash_map, HashMap};

// imgs, anims, sounds
#[derive(Clone, Default)]
pub struct ResourceConfig {
    pub imgs: Vec<(String, String)>,
    pub anims: Vec<(String, String, (usize, f64))>,
    pub sounds: Vec<(String, String)>,
    pub fonts: Vec<(String, String)>,
}

pub struct Resources {
    pub imgs: HashMap<String, Image>,
    pub anims: HashMap<String, Animation>,
    pub sounds: HashMap<String, Sound>,
    pub fonts: HashMap<String, (Font, HashMap<String, Image>)>,
}

impl Resources {
    pub fn new(
        ResourceConfig {
            imgs,
            anims,
            sounds,
            fonts,
        }: ResourceConfig,
    ) -> impl Future<Item = Self, Error = Error> {
        let img_futs = join_all(imgs.into_iter().map(move |(name, src)| {
            load_file(src.to_owned())
                .map(move |data| (name, Image::from_bytes(data.as_slice()).unwrap()))
        }))
        .map(|vec| vec.into_iter().collect());

        let anim_futs = join_all(anims.into_iter().map(move |(name, src, dims)| {
            load_file(src.to_owned())
                .map(move |data| (name, (dims, Image::from_bytes(data.as_slice()).unwrap())))
        }))
        .map(|vec| {
            vec.into_iter()
                .map(|(src, ((nframes, dur), img))| {
                    let anim = Animation::from_image(img, nframes, dur);
                    (src.to_string(), anim)
                })
                .collect()
        });

        let sound_futs = join_all(
            sounds
                .into_iter()
                .map(|(name, src)| Sound::load(src.to_owned()).map(move |sound| (name, sound))),
        )
        .map(|vec| vec.into_iter().collect());

        let font_futs = join_all(
            fonts
                .into_iter()
                .map(|(name, src)| Font::load(src).map(move |font| (name, font))),
        )
        .map(|vec| {
            vec.into_iter()
                .map(|(name, font)| (name, (font, HashMap::new())))
                .collect::<HashMap<_, _>>()
        });

        anim_futs
            .join4(img_futs, sound_futs, font_futs)
            .map(|(anims, imgs, sounds, mut fonts)| {
                fonts.entry("default".to_owned()).or_insert_with(|| {
                    (
                        Font::from_slice(include_bytes!("../../include/VGATypewriter.ttf"))
                            .unwrap(),
                        HashMap::new(),
                    )
                });
                Resources {
                    imgs,
                    anims,
                    sounds,
                    fonts,
                }
            })
    }

    pub fn get_img(&self, name: &str) -> Option<&Image> {
        self.imgs.get(name)
    }

    pub fn get_sound(&self, name: &str) -> Option<&Sound> {
        self.sounds.get(name)
    }
}

impl Resources {
    pub fn get_anim(&self, name: &str) -> Option<&Animation> {
        self.anims.get(name)
    }

    #[allow(unused)]
    pub fn get_anim_mut(&mut self, name: &str) -> Option<&mut Animation> {
        self.anims.get_mut(name)
    }

    pub fn update_anim(&mut self, window: &mut Window) -> Result<()> {
        for i in self.anims.values_mut() {
            i.update(window)?;
        }
        Ok(())
    }

    pub fn render_str<'a>(
        &'a mut self,
        font_name: Option<&str>,
        s: &str,
        style: FontStyle,
        store_in_cache: bool,
    ) -> Result<Option<Cow<'a, Image>>> {
        let font_name = font_name.unwrap_or("default");

        let (font, cache) = match self.fonts.get_mut(font_name) {
            Some(f) => f,
            None => return Ok(None),
        };

        let render = || font.render(s, &style);

        let ret = if store_in_cache {
            // TODO: don't clone the string here
            let cached = match cache.entry(s.to_owned()) {
                hash_map::Entry::Occupied(o) => o.into_mut(),
                hash_map::Entry::Vacant(v) => v.insert(render()?),
            };
            Cow::Borrowed(cached)
        } else {
            Cow::Owned(render()?)
        };
        Ok(Some(ret))
    }
}
