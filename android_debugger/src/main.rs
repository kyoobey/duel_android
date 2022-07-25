


#[macro_use]
extern crate log;
use simple_logger;

use clappers;
use lazy_static::lazy_static;
use regex::Regex;

use std::{
	env,
	path::Path,
	process::{ Command, Stdio },
	io::{ BufReader, BufRead, Write },
};



fn main() -> anyhow::Result<()>{

	let clappers = clappers::Clappers::build()
		.set_flags(vec![
			"h|help",
			"t|time",
			"r|rust"
		])
		.set_singles(vec![
			"v|loglevel",
			"p|package"
		])
		.parse();

	if clappers.get_flag("help") {
		println!("\
			A utility to debug native android apps in rust \n\n\
			ANDROID_NDK_HOME and ANDROID_SDK_ROOT need to be setup to the appropriate paths and \
			this command should be run project root (where target is accessible at ./target)\n\
			usage: debug [arguments]\n\
			Arguments:\n\
				\t-h|--help     Print this help message\n\
				\t-v|--loglevel <n> Changes the log level 0 (lowest) to 3 (highest)\n\
				\t-p|--package <n> The android package name (required)\n\
				\t-t|--time <n> outputs the time with debug info\n\
				\t-r|--rust <n> outputs only the rust stacktrace\n\
		");
		std::process::exit(0);
	}



	// setup logging
	simple_logger::SimpleLogger::new()
		.with_level(match clappers.get_single("loglevel").parse()
			.unwrap_or_else(|_err| 0) {
			0 => log::LevelFilter::Error,
			1 => log::LevelFilter::Warn,
			2 => log::LevelFilter::Info,
			3 => log::LevelFilter::Debug,
			_ => {
				println!("[Warning] Log Level should be in the range 0..3");
				log::LevelFilter::Error
			}
		})
		.init()
		.expect("Couldn't initialize logger");


	let mut package_name = clappers.get_single("package");
	if package_name.len() < 1 {
		warn!("No package name provided, using \"com.tshrpl.duel\"");
		package_name = String::from("com.thrpl.duel");
	}

	let android_sdk_path = env::var("ANDROID_SDK_ROOT")
		.expect("[Error] ANDROID_SDK_ROOT env var not set");
	debug!("ANDROID_SDK_ROOT = {}", android_sdk_path);
	let android_ndk_path = env::var("ANDROID_NDK_HOME")
		.expect("[Error] ANDROID_NDK_HOME env var not set");
	debug!("ANDROID_NDK_HOME = {}", android_ndk_path);
	


	let adb_path = Path::new(&android_sdk_path)
		.join("platform-tools")
		.join( if cfg!(windows) { "adb.exe" } else { "adb" } );
	debug!("adb: {:?}", adb_path);

	// let addr2line_path = Path::new(&android_ndk_path)
	//     .join( if cfg!(windows) {
	//         r"toolchains\llvm\prebuilt\windows-x86_64\bin\aarch64-linux-android-addr2line.exe"
	//     } else {
	//         "toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android-addr2line"
	//     });
	// windows
	let addr2line_path = Path::new(&android_ndk_path).join(r"toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-addr2line.exe");
	debug!("ndk addr2line: {:?}", addr2line_path);


	let logcat_stdout = Command::new(&adb_path)
				.arg("logcat")
				// .arg("-b")
				// .arg("main") // "radio" or "events"
				.stdout(Stdio::piped())
				.spawn().expect("[Error] Couldn't run logcat")
				.stdout.expect("[Error] Couldn't get logcat stdout");
	let logcat_reader = BufReader::new(logcat_stdout);

	let debugfile = std::env::current_dir().ok().unwrap().join(if cfg!(windows) {r"target\aarch64-linux-android\debug\libmain.so"} else {"target/aarch64-linux-android/debug/libmain.so"});
	let debugfile = debugfile.to_str().unwrap();
	debug!("Debug file to analyze: {:?}", debugfile);

	let mut addr2line_process = Command::new(addr2line_path)
				.arg("-f")
				.arg("-C")
				.arg("-e")
				.arg(debugfile)
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.spawn().expect("[Error] Couldn't run addr2line");
	let mut addr2line_stdin = addr2line_process
				.stdin.take().expect("[Error] Couldn't get addr2line stdin");
	let addr2line_stdout = addr2line_process
				.stdout.take().expect("[Error] Couldn't get addr2line stdout");
	let mut addr2line_reader = BufReader::new(addr2line_stdout);


	lazy_static! {
		static ref REGEX_WHOLE: Regex = Regex::new(r"(\d+-\d+\s+\d+:\d+:\d+\.\d+)\s+\d+\s+\d+\s+(\w)\s+(.+)").expect("[Error] Couldn't compile regex");
		static ref REGEX_ADDRESS: Regex = Regex::new(r".*#\d+\s+\w+\s+(.*)\s+.*").expect("[Error] Couldn't compile regex");
	}

	let time = clappers.get_flag("time");
	let level: u8 = clappers.get_single("loglevel").parse().unwrap_or_else(|_err| 0);

	let REGEX_PS: Regex = Regex::new(&format!(r"\w+\s+(\d+).*{}", package_name)).expect("[Error] Couldn't compile regex");
	// let ps_output = std::str::from_utf8(&Command::new(&adb_path).args(["shell", "ps"]).output().expect("Couldn't run adb ps").stdout[..]).ok().unwrap();
	let ps_output = &Command::new(&adb_path).args(["shell", "ps"]).output().expect("Couldn't run adb ps").stdout[..];
	let ps_output = std::str::from_utf8(ps_output).ok().unwrap();
	let mut ps = String::new();
	for l in REGEX_PS.captures_iter(&ps_output) {
		ps = String::from(&l[1]);
		debug!("Package process id: {:?}", &ps);
	}

	for line in logcat_reader.lines() {
		let line = line.ok().unwrap_or_else(|| String::from(""));
		if line.contains(&ps[..]) {
			// println!("{}", line);
			for l in REGEX_WHOLE.captures_iter(&line) {
				let timestamp = if time { format!("{} ", &l[1]) } else { String::from("") };
				let fout = format!("{}{}", &timestamp, &l[3]);

				if !clappers.get_flag("rust") {
					if level >= 3 && (&l[2] == "D" || &l[2] == "V" || &l[2] == "S") {
						debug!("{}", fout);
					} if level >= 2 && &l[2] == "I" {
						info!("{}", fout);
					} if level >= 1 && &l[2] == "W" {
						warn!("{}", fout);
					} if &l[2] == "E" {
						error!("{}", fout);
					} if &l[2] == "F" {
						error!("{}", fout);
						if l[3].contains("offset") {
							for j in REGEX_ADDRESS.captures_iter(&l[3]) {
								// error!("{:?}", &j[1]);  // i want j[1] to be piped into another
								addr2line_stdin.write_all(&j[1].as_bytes())?;
								addr2line_stdin.write_all(b"\n")?;

								let mut buf = String::from("");
								addr2line_reader.read_line(&mut buf)?;
								buf.pop();
								error!("[Rust stacktrace] {}", buf);
							}
						}
					}
				} else {
					if &l[2] == "F" {
						if l[3].contains("offset") {
							for j in REGEX_ADDRESS.captures_iter(&l[3]) {
								// error!("{:?}", &j[1]);  // i want j[1] to be piped into another
								addr2line_stdin.write_all(&j[1].as_bytes())?;
								addr2line_stdin.write_all(b"\n")?;

								let mut buf = String::from("");
								addr2line_reader.read_line(&mut buf)?;
								buf.pop();
								error!("[Rust stacktrace] {}", buf);
							}
						}
					}
				}
			}
		}
	}

	addr2line_process.wait()?;

	Ok(())
}
