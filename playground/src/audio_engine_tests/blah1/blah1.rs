


#[macro_use]
extern crate log;
use simple_logger::SimpleLogger;

use audio_engine::{ AudioEngine, WavDecoder };

use std::{
	io::Cursor,
	time::Instant,
	sync::{
		Arc,
		atomic::{ AtomicU32, Ordering }
	}
};



fn main () {

	SimpleLogger::new().with_level(log::LevelFilter::Trace).init().unwrap();

	let engine = AudioEngine::new().unwrap();
	let mut intro_music = engine
							.new_sound(WavDecoder::new(Cursor::new(&include_bytes!("intro.wav")[..])).unwrap(), |x| x)
							.unwrap();

	// intro_music.set_loop(true);
	intro_music.play();

	let mut s1 = engine
					.new_sound(WavDecoder::new(Cursor::new(&include_bytes!("sin_500hz.wav")[..])).unwrap(), |x| x)
					.unwrap();

	s1.set_loop(true);
	s1.play();
	s1.set_volume(1.0);

	let start_time = Instant::now();

	// let max = Arc::new(AtomicU32::new(0));
	// let min = Arc::new(AtomicU32::new(9999999));

	loop {
		let t = (Instant::now() - start_time).as_secs_f32() % 4.0;
		// debug!("time: {}", time);
		// std::thread::sleep(std::time::Duration::from_secs(1));
		// let v = time.sin();
		// s1.set_volume(0.0);

		// let max = max.clone();
		// let min = min.clone();
		s1.effect(move |x| {
			// if x > f32::from_bits(max.load(Ordering::Relaxed)) {
			// 	println!("{}", x);
			// 	max.store(x.to_bits(), Ordering::Relaxed);
			// }
			// if x < f32::from_bits(min.load(Ordering::Relaxed)) {
			// 	println!("{}", x);
			// 	min.store(x.to_bits(), Ordering::Relaxed);
			// }
			// x * (((-4.0*(t-1.0).powf(2.0)) as f32).exp() + 0.5*((-3.0*(t-3.0).powf(2.0)) as f32).exp())
			// x * ((44800.0*t).sin() * 0.5 + 0.5)
			0.0
		});

		if t>10.0 {
			drop(s1);
			drop(intro_music);
			break;
		}
	}

}


