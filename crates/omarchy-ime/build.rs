// Give the cdylib a SONAME. Without one, anything that links against it (the Fcitx5
// front end) records the library's absolute build path, e.g.
// /tmp/omarchy-ime.XXXX/target/release/libomarchy_ime.so, as NEEDED. install.sh
// deletes that build tree, so the installed addon could not be loaded and the input
// method never appeared in Fcitx5. With the SONAME the front end needs
// "libomarchy_ime.so" and finds it next to itself through its $ORIGIN RUNPATH.
fn main() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libomarchy_ime.so");
}
