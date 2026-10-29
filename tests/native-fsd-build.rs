// License: MIT OR Apache-2.0

//! Builds WDK-typed native host observers; no kernel import library is linked.

fn main() {
    let config = wdk_build::Config::from_env_auto().expect("fixture has a WDK driver model");
    let mut build = cc::Build::new();
    for (key, value) in config.preprocessor_definitions() {
        build.define(&key, value.as_deref());
    }
    build
        .include("../../crates/wdk-sys/include")
        .file("../native-fsd-boundary-support.c")
        .flag("/W4")
        .flag("/WX");
    let compiler = build.get_compiler();
    if compiler.is_like_clang_cl() {
        build.flag("/EHa");
        for include in config.include_paths().expect("WDK include paths") {
            build.flag(format!("-imsvc{}", include.display()));
        }
    } else {
        build.includes(config.include_paths().expect("WDK include paths"));
        build.flag("/external:anglebrackets").flag("/external:W0");
    }
    build.compile("native_fsd_boundary_support");
    println!("cargo:rerun-if-changed=../native-fsd-boundary-support.c");
    println!("cargo:rerun-if-changed=../../crates/wdk-sys/include/wdk_file_system.h");
}
