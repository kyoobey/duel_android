


# android dependencies

set `ANDROID_SDK_ROOT`
Android SDK buildtools 31

set `ANDROID_NDK_HOME`
Android NDK 25...



# dependencies
- https://github.com/rib/android-activity
- https://github.com/rib/winit
- https://github.com/gfx-rs/wgpu



# testing
build crate `game` with feature `desktop`

```bash
cargo run --features=duel/desktop gametest
```



# testing on android

cd into `game` directory
```bash
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/ build
```
```
.\gradlew build
```
```
.\gradlew installDebug
```
```
adb shell am start -n com.tshrpl.duel/.MainActivity
```


