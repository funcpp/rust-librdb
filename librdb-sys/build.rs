use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let librdb_root = manifest_dir
        .join("librdb")
        .canonicalize()
        .expect("librdb-sys/librdb not found — run `git submodule update --init`");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!(
        "cargo:rerun-if-changed={}",
        librdb_root.join("api/librdb-api.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        librdb_root.join("api/librdb-ext-api.h").display()
    );
    for dir in &[
        librdb_root.join("src/lib"),
        librdb_root.join("src/ext"),
        librdb_root.join("deps/redis"),
    ] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "c" || e == "h") {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }

    let dynamic = env::var("CARGO_FEATURE_DYNAMIC_LINKING").is_ok();
    let r#static = env::var("CARGO_FEATURE_STATIC_LINKING").is_ok();

    if dynamic && r#static {
        panic!("features `dynamic-linking` and `static-linking` are mutually exclusive");
    }

    if dynamic {
        println!("cargo:rustc-link-lib=rdb");
    } else if r#static {
        let root = env::var("DEP_LIBRDB_STATIC_ROOT")
            .expect("static-linking requires DEP_LIBRDB_STATIC_ROOT env var");
        println!("cargo:rustc-link-search=native={root}");
        println!("cargo:rustc-link-lib=static=rdb");
    } else {
        build_from_source(&librdb_root, &out_dir);
    }
}

fn build_from_source(librdb_root: &Path, out_dir: &Path) {
    let lib_src = librdb_root.join("src/lib");
    let ext_src = librdb_root.join("src/ext");
    let redis_deps = librdb_root.join("deps/redis");

    let mut build = cc::Build::new();
    let _ = build
        .warnings(false)
        .flag("-std=c99")
        .flag("-fvisibility=hidden")
        .define("NDEBUG", "1")
        .include(&lib_src)
        .include(&ext_src)
        .include(librdb_root.join("src"))
        .include(librdb_root.join("api"))
        .include(&redis_deps);

    for entry in std::fs::read_dir(&lib_src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "c") {
            let _ = build.file(&path);
        }
    }

    let ext_includes = [
        "extCommon.c",
        "handlersFilter.c",
        "handlersToJson.c",
        "handlersToPrint.c",
        "handlersToResp.c",
        "readerFile.c",
        "readerFileDesc.c",
    ];
    for name in &ext_includes {
        let _ = build.file(ext_src.join(name));
    }

    for entry in std::fs::read_dir(&redis_deps).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "c") {
            let _ = build.file(&path);
        }
    }

    build.compile("rdb");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=rdb");
}
