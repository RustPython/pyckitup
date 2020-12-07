use crate::prelude::*;
use std::collections::{hash_map, HashMap};

// imgs, anims, sounds
#[derive(Clone, Default)]
pub struct ResourceConfig {
    pub imgs: Vec<(String, String)>,
    pub anims: Vec<(String, String, (usize, f64))>,
    pub sounds: Vec<(String, String)>,
    pub fonts: Vec<(String, String, f32)>,
}

pub struct Resources {
    pub imgs: HashMap<String, Image>,
    pub anims: HashMap<String, Animation>,
    pub sounds: HashMap<String, Sound>,
    pub fonts: HashMap<String, (FontRenderer, f32)>,
}

impl Resources {
    pub async fn new(
        ResourceConfig {
            imgs,
            anims,
            sounds,
            fonts,
        }: ResourceConfig,
        gfx: &Graphics,
    ) -> anyhow::Result<Self> {
        let img_futs = future::try_join_all(
            imgs.into_iter()
                .map(|(name, src)| async move { Ok((name, Image::load(gfx, src).await?)) }),
        );

        let anim_futs = future::try_join_all(anims.into_iter().map(
            |(name, src, (nframes, dur))| async move {
                let image = Image::load(gfx, src).await?;
                let anim = Animation::from_image(image, nframes, dur);
                Ok((name, anim))
            },
        ));

        let sound_futs = future::try_join_all(sounds.into_iter().map(|(name, src)| async move {
            let sound = Sound::load(src).await?;
            Ok((name, sound))
        }));

        let font_futs =
            future::try_join_all(fonts.into_iter().map(|(name, src, size)| async move {
                let font = VectorFont::load(src).await?.to_renderer(gfx, size)?;
                Ok::<_, anyhow::Error>((name, (font, size)))
            }));

        let (anims, imgs, sounds, fonts) =
            futures::try_join!(anim_futs, img_futs, sound_futs, font_futs)?;
        let anims = anims.into_iter().collect();
        let imgs = imgs.into_iter().collect();
        if !sounds.is_empty() {
            Sound::init();
        }
        let sounds = sounds.into_iter().collect();
        let mut fonts = fonts.into_iter().collect::<HashMap<_, _>>();
        if let hash_map::Entry::Vacant(v) = fonts.entry("default".to_owned()) {
            v.insert((
                VectorFont::from_slice(include_bytes!("../../include/VGATypewriter.ttf"))
                    .to_renderer(gfx, 10.0)?,
                10.0,
            ));
        }
        Ok(Resources {
            imgs,
            anims,
            sounds,
            fonts,
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

    pub fn update_anim(&mut self, update_rate: f64) {
        for i in self.anims.values_mut() {
            i.update(update_rate);
        }
    }

    pub fn get_font(&mut self, font_name: &str) -> Option<(&mut FontRenderer, f32)> {
        self.fonts.get_mut(font_name).map(|(f, pt)| (f, *pt))
    }
}
