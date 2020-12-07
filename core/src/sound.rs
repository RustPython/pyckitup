//! sound implementation taken from quicksilver 0.3

use std::path::Path;

#[derive(Clone)]
pub struct Sound {
    val: audio::AudioData,
    volume: f32,
}

#[cfg(not(target_arch = "wasm32"))]
mod audio {
    use super::*;
    use once_cell::unsync::OnceCell;
    use std::io::Cursor;
    use std::sync::Arc;

    std::thread_local! {
        static STREAM: OnceCell<(rodio::OutputStream, rodio::OutputStreamHandle)> =
            OnceCell::new();
    }
    #[derive(Clone)]
    pub struct AudioData(Arc<Vec<u8>>);

    impl AsRef<[u8]> for AudioData {
        fn as_ref(&self) -> &[u8] {
            &self.0
        }
    }
    impl AudioData {
        pub async fn load(path: &Path) -> anyhow::Result<Self> {
            let val = AudioData(std::fs::read(path)?.into());
            rodio::Decoder::new(Cursor::new(val.clone()))?;
            Ok(val)
        }
        pub fn play(&self, volume: f32) -> anyhow::Result<()> {
            STREAM.with(|stream| {
                let (_, stream_handle) =
                    stream.get_or_try_init(rodio::OutputStream::try_default)?;
                let sink = stream_handle.play_once(Cursor::new(self.clone()))?;
                sink.set_volume(volume);
                sink.detach();
                Ok(())
            })
        }
    }

    pub fn init() {
        STREAM.with(|stream| {
            if let Ok((_, handle)) = stream.get_or_try_init(rodio::OutputStream::try_default) {
                let _ = handle.play_raw(rodio::source::Empty::new());
            }
        })
    }
}

#[cfg(target_arch = "wasm32")]
mod audio {
    use super::*;
    use anyhow::Context;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    #[wasm_bindgen(module = "/src/sound.js")]
    extern "C" {
        fn load(path: &str) -> js_sys::Promise;

        #[derive(Clone)]
        pub type AudioData;
        #[wasm_bindgen(method, catch)]
        fn _play(this: &AudioData, volume: f32) -> Result<(), JsValue>;
    }

    impl AudioData {
        pub async fn load(path: &Path) -> anyhow::Result<Self> {
            let fut = JsFuture::from(load(path.to_str().context("audio path is not utf8")?));
            let val = fut.await.map_err(jserr)?.unchecked_into();
            Ok(val)
        }
        pub fn play(&self, volume: f32) -> anyhow::Result<()> {
            self._play(volume).map_err(jserr)?;
            Ok(())
        }
    }

    fn jserr(v: JsValue) -> anyhow::Error {
        let s = String::from(v.unchecked_ref::<js_sys::Object>().to_string());
        anyhow::Error::msg(s)
    }

    pub fn init() {}
}

impl Sound {
    /// Start loading a sound from a given path
    #[inline]
    pub async fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let val = audio::AudioData::load(path.as_ref()).await?;
        Ok(Self { val, volume: 1.0 })
    }

    /// Get the volume of the sound clip instance
    ///
    /// The volume is multiplicative, meaing 1 is the identity, 0 is silent, 2 is twice the
    /// amplitude, etc. Note that sound is not perceived linearly so results may not correspond as
    /// expected.
    #[allow(unused)] // TODO: do something with this
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Set the volume of the sound clip instance
    ///
    /// The volume is multiplicative, meaing 1 is the identity, 0 is silent, 2 is twice the
    /// amplitude, etc. Note that sound is not perceived linearly so results may not correspond as
    /// expected.
    #[allow(unused)] // TODO: do something with this
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }

    /// Play the sound clip at its current volume
    ///
    /// The sound clip can be played over itself.
    ///
    /// Future changes in volume will not change the sound emitted by this method.
    pub fn play(&self) -> anyhow::Result<()> {
        self.val.play(self.volume)
    }

    pub(crate) fn init() {
        audio::init();
    }
}
