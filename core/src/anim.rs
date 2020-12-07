use crate::prelude::*;

pub struct Animation {
    image: Image,
    pub played: bool,
    nframes: usize,
    /// in seconds
    duration: f64,
    current_t: f64,
    pub frame_size: Vector,
}

impl Animation {
    pub fn from_image(image: Image, nframes: usize, duration: f64) -> Animation {
        let mut frame_size = image.size();
        frame_size.x /= nframes as f32;

        Animation {
            image,
            played: false,
            nframes,
            duration,
            current_t: 0.,
            frame_size,
        }
    }

    pub fn update(&mut self, update_rate: f64) {
        self.current_t += update_rate / 1000.0;
        if self.current_t >= self.duration {
            self.current_t -= self.duration
        }

        if self.nth() == self.nframes - 1 {
            self.played = true;
        }
    }

    pub fn nth(&self) -> usize {
        let frame = (self.current_t / self.duration * self.nframes as f64).floor() as usize + 1;
        frame % self.nframes
    }

    pub fn draw(&self, gfx: &mut Graphics, location: Rectangle) {
        let n = self.nth();

        let region = Rectangle::new(self.frame_size.x_comp() * n as f32, self.frame_size);

        gfx.draw_subimage(&self.image, region, location);
    }

    #[allow(unused)]
    pub fn play(&mut self) -> QsResult<()> {
        self.played = false;
        self.current_t = 0.;
        Ok(())
    }

    pub fn set_duration(&mut self, duration: f64) {
        self.duration = duration;
    }
}
