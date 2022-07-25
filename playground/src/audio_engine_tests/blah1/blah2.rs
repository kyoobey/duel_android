
use std::sync::{ Arc, Mutex };

struct Sound {
	data: [f32; 4],
	effect: Box<dyn Fn(f32) -> f32 + Send>
}

impl Sound {
	fn new (data: [f32; 4], effect: impl Fn(f32) -> f32 + 'static + std::marker::Send) -> Self {
		Self { data, effect: Box::new(effect) }
	}
}


struct Mixer {
	sounds: Vec<Sound>
}

impl Mixer {
	fn new () -> Self {
		Self { sounds: vec![] }
	}

	fn add_sound (&mut self, sound: Sound) {
		self.sounds.push(sound);
	}

	fn samples (&self) -> [f32; 4] {
		self.sounds[0].data.map(|x| (self.sounds[0].effect)(x))
	}
}


struct AudioEngine {
	mixer: Arc<Mutex<Mixer>>
}

impl AudioEngine {
	fn new () -> Self {
		let mixer = Arc::new(Mutex::new(Mixer::new()));

		Self {
			mixer
		}
	}

	fn start (&self) {
		let mixer = self.mixer.clone();

		std::thread::spawn(move || {
			let mixer = mixer.lock().unwrap();
			loop{
				for i in mixer.samples() {
					println!("sample {}", i);
				}
			}
		});
	}

	fn new_sound (&mut self, data: [f32; 4], effect: impl Fn(f32) -> f32 + 'static + std::marker::Send) {
		self.mixer.lock().unwrap().add_sound(Sound::new(data, effect));
	}
}


fn main () {

	// let sound1 = Sound::new([1., 2., 3., 4.], |x| x*0.5);
	// println!("{:?}", sound1.samples());

	let PI = std::f64::consts::PI as f32;

	let mut engine = AudioEngine::new();
	engine.new_sound([0., PI/4., PI/2., PI], |x| x.sin());
	engine.start();

	loop {}

}
