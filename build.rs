fn main() {
    // C compilation is only needed for the bench_compare benchmark (--features c-bench).
    // The vendor/libdivsufsort git submodule must be checked out when this feature is active.
    #[cfg(feature = "c-bench")]
    if std::env::var("CARGO_FEATURE_C_BENCH").is_ok() {
        cc::Build::new()
            .files([
                "vendor/libdivsufsort/lib/divsufsort.c",
                "vendor/libdivsufsort/lib/sssort.c",
                "vendor/libdivsufsort/lib/trsort.c",
                "vendor/libdivsufsort/lib/utils.c",
            ])
            // vendor/include: our generated config.h and divsufsort.h (cmake template substitutes).
            // vendor/libdivsufsort/include: divsufsort_private.h from the upstream submodule.
            // vendor/include is listed first so our config.h takes precedence.
            .include("vendor/include")
            .include("vendor/libdivsufsort/include")
            // Enable the same optimization flags used by the default cmake Release build.
            .opt_level(3)
            .define("HAVE_CONFIG_H", "1")
            .define("PROJECT_VERSION_FULL", "\"2.0.2\"")
            .compile("divsufsort_c");

        println!("cargo:rerun-if-changed=vendor/include");
        println!("cargo:rerun-if-changed=vendor/libdivsufsort");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
