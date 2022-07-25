


use crate::converter;

use std::sync::{
	Arc,
	Mutex,
	atomic::{ AtomicU64, Ordering }
};



type SoundId = u64;



fn next_id() -> SoundId {
	static GLOBAL_COUNT: AtomicU64 = AtomicU64::new(0);
	GLOBAL_COUNT.fetch_add(1, Ordering::Relaxed)
}



/// the number of samples processed per second for a single channel of audio
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(pub u32);



/// represents a sound in the audio engine. if this is dropped,
/// the sound will continue to play until it ends.
pub struct Sound {

	pub mixer: Arc<Mutex<Mixer>>,
	pub id: SoundId

}

impl Sound {


	/// starts or continue to play the sound
	///
	/// if the sound was paused ot stopped, it will start playing
	/// again. otherwise, does nothing
	pub fn play (&mut self) {
		self.mixer.lock().unwrap().play(self.id);
	}


	/// pause the sound
	///
	/// if the sound is playing, it will pause. if play is called,
	/// this sound will continue from where it was before pause.
	/// if the sound is not playing, doesn nothing.
	pub fn pause (&mut self) {
		self.mixer.lock().unwrap().pause(self.id);
	}


	/// stop the sound
	///
	/// if the sound is playing, it will pause and reset the song.
	/// when play is called, this sound will start from beggining.
	/// even if the sound is not playing, it will reset the sound.
	pub fn stop (&mut self) {
		self.mixer.lock().unwrap().stop(self.id);
	}


	/// reset the sound to the start
	///
	/// the behaviour is the same being the sound playing or not
	pub fn reset (&mut self) {
		self.mixer.lock().unwrap().reset(self.id);
	}


	/// set the volume of the sound
	pub fn set_volume(&mut self, volume: f32) {
		self.mixer.lock().unwrap().set_volume(self.id, volume);
	}


	/// set if the sound will repeat every time it reaches the end
	pub fn set_loop (&mut self, looping: bool) {
		self.mixer.lock().unwrap().set_loop(self.id, looping);
	}


	/// update sound effect
	pub fn effect (&mut self, effect: impl FnMut(f32) -> f32 + 'static + std::marker::Send) {
		self.mixer.lock().unwrap().update_effect(self.id, effect);
	}


}

impl Drop for Sound {
	fn drop (&mut self) {
		self.mixer.lock().unwrap().drop_sound(self.id);
	}
}



/// a source of sound samples
///
/// sound samples of each channel must be interleaved
pub trait SoundSource {

	/// return the number of channels
	fn channels (&self) -> u16;

	/// return the sample rate
	fn sample_rate (&self) -> u32;

	/// start the sound from the beggining
	fn reset (&mut self);

	/// write the samples to `buffer`
	///
	/// return how many samples was written. if it returns a value
	/// less than the length of `buffer`, this indicates that the
	/// sound has ended.
	///
	/// the `buffer` length and the returned length should always be
	/// a multiple of [`self.channels()`](SoundSource::channels).
	fn write_samples (&mut self, buffer: &mut [i16]) -> usize;

}

impl<T: SoundSource + ?Sized> SoundSource for Box<T> {

	fn channels (&self) -> u16 {
		(**self).channels()
	}

	fn sample_rate (&self) -> u32 {
		(**self).sample_rate()
	}

	fn reset (&mut self) {
		(**self).reset()
	}

	fn write_samples (&mut self, buffer: &mut [i16]) -> usize {
		(**self).write_samples(buffer)
	}

}


struct SoundInner {

	id: SoundId,
	data: Box<dyn SoundSource + Send>,
	volume: f32,
	looping: bool,
	drop: bool,
	effect: Box<dyn FnMut(f32) -> f32 + Send>

}

impl SoundInner {

	fn new (data: Box<dyn SoundSource + Send>, effect: impl FnMut(f32) -> f32 + 'static + std::marker::Send) -> Self {
		Self {
			id: next_id(),
			data,
			volume: 1.0,
			looping: false,
			drop: false,
			effect: Box::new(effect)
		}
	}

}



/// keep track of each Sound, and mix their output together
pub struct Mixer {

	sounds: Vec<SoundInner>,
	playing: usize,
	pub channels: u16,
	pub sample_rate: SampleRate

}

impl Mixer {


	pub fn new (channels: u16, sample_rate: SampleRate) -> Self {
		Self {
			sounds: vec![],
			playing: 0,
			channels,
			sample_rate
		}
	}


	/// change the number of channels and the sample rate
	///
	/// this will also keep all currently playing sounds and convert
	/// them to the new config if necessary
	pub fn set_config (&mut self, channels: u16, sample_rate: SampleRate) {

		struct Nop;
		#[rustfmt::skip]
		impl SoundSource for Nop {
			fn channels (&self) -> u16 { 0 }
			fn sample_rate (&self) -> u32 { 0 }
			fn reset (&mut self) { }
			fn write_samples (&mut self, _: &mut [i16]) -> usize { 0 }
		}

		let not_changed = self.channels == channels && self.sample_rate == sample_rate;
		if not_changed {
			return;
		}
		if !self.sounds.is_empty() {
			for sound in self.sounds.iter_mut() {
				// https://github.com/Rodrigodd/audio-engine/blob/3d0da3711b5cc78e7192d616ebb1d4069920707d/src/lib.rs#L200
				// Beware !! read the link
				if sound.data.channels() != channels {
					let inner = std::mem::replace(&mut sound.data, Box::new(Nop));
					sound.data = Box::new(converter::ChannelConverter::new(inner, channels));
				}
				if sound.data.sample_rate() != sample_rate.0 {
					let inner = std::mem::replace(&mut sound.data, Box::new(Nop));
					sound.data = Box::new(converter::SampleRateConverter::new(inner, sample_rate.0));
				}
			}
		}
		self.channels = channels;
		self.sample_rate = sample_rate;

	}


	pub fn add_sound (&mut self, sound: Box<dyn SoundSource + Send>, effect: impl FnMut(f32) -> f32 + 'static + std::marker::Send) -> SoundId {
		let sound_inner = SoundInner::new(sound, effect);
		let id = sound_inner.id;
		self.sounds.push(sound_inner);
		id
	}


	/// if the sound was paused ot stopped, it will start playing
	/// again. otherwise, does nothing
	pub fn play (&mut self, id: SoundId) {
		for i in (self.playing..self.sounds.len()).rev() {
			if self.sounds[i].id == id {
				self.sounds.swap(self.playing, i);
				self.playing += 1;
				break;
			}
		}
	}


	/// if the sound is playing, it will pause. if play is called,
	/// this sound will continue from where it was when pause.
	/// if the sound is not playing, does nothing
	pub fn pause (&mut self, id: SoundId) {
		for i in (0..self.playing).rev() {
			if self.sounds[i].id == id {
				self.playing -= 1;
				self.sounds.swap(self.playing, i);
				break;
			}
		}
	}


	/// if the sound is playing, it will pause and reset the song.
	/// when play is called this sound will start from the beggining
	/// even if the sound is not playing, it will reset the sound to
	/// the start
	pub fn stop (&mut self, id: SoundId) {
		for i in (0..self.sounds.len()).rev() {
			if self.sounds[i].id == id {
				self.sounds[i].data.reset();
				if i < self.playing {
					self.playing -= 1;
					self.sounds.swap(self.playing, i);
				}
				break;
			}
		}
	}


	/// this reset the sound to the start, the sound being played
	/// or not
	pub fn reset (&mut self, id: SoundId) {
		for i in (0..self.sounds.len()).rev() {
			if self.sounds[i].id == id {
				self.sounds[i].data.reset();
				break;
			}
		}
	}


	/// set the volume of the sound
	pub fn set_volume (&mut self, id: SoundId, volume: f32) {
		for i in (0..self.sounds.len()).rev() {
			if self.sounds[i].id == id {
				self.sounds[i].volume = volume;
				break;
			}
		}
	}


	/// set if the sound will repeat ever time it reach the end
	pub fn set_loop (&mut self, id: SoundId, looping: bool) {
		for i in (0..self.sounds.len()).rev() {
			if self.sounds[i].id == id {
				self.sounds[i].looping = looping;
				break;
			}
		}
	}


	/// mark the sound to be dropped after it reaches the end
	pub fn drop_sound (&mut self, id: SoundId) {
		for i in (0..self.sounds.len()).rev() {
			if self.sounds[i].id == id {
				self.sounds[i].drop = true;
				break;
			}
		}
	}


	/// update sound effect
	pub fn update_effect (&mut self, id: SoundId, effect: impl FnMut(f32) -> f32 + 'static + std::marker::Send) {
		for i in (0..self.sounds.len()).rev() {
			if self.sounds[i].id == id {
				self.sounds[i].effect = Box::new(effect);
				break;
			}
		}
	}


}

impl SoundSource for Mixer {


	fn channels (&self) -> u16 {
		self.channels
	}


	fn sample_rate (&self) -> u32 {
		self.sample_rate.0
	}


	fn reset (&mut self) {}


	fn write_samples (&mut self, buffer: &mut [i16]) -> usize {

		if self.playing == 0 {
			for b in buffer.iter_mut() {
				*b = 0;
			}
			return buffer.len();
		}

		let mut buf = vec![0; buffer.len()];
		let mut s = 0;
		while s < self.playing {
			let mut len = 0;
			loop {
				len += self.sounds[s].data.write_samples(&mut buf[len..]);
				if len < buffer.len() {
					self.sounds[s].data.reset();
					if self.sounds[s].looping {
						continue;
					}
				}
				break;
			}

			if (self.sounds[s].volume - 1.0).abs() < 1.0 / i16::max_value() as f32 {
				for i in 0..len {
					buffer[i] = buffer[i].saturating_add((self.sounds[s].effect)(buf[i] as f32) as i16);
				}
			} else {
				for i in 0..len {
					buffer[i] = buffer[i].saturating_add(((self.sounds[s].effect)(buf[i] as f32) * self.sounds[s].volume) as i16);
				}
			}

			if len < buffer.len() {
				if self.sounds[s].drop {
					let _ = self.sounds.swap_remove(s);
				}
				self.playing -= 1;
				if self.playing > 0 && self.playing < self.sounds.len() {
					self.sounds.swap(s, self.playing);
				} else {
					break;
				}
			} else {
				s += 1;
			}
		}

		buffer.len()

	}


}


