fn main() {
    // C compilation is only needed for the bench_compare benchmark (--features c-bench).
    // The vendor/libdivsufsort git submodule must be checked out when this feature is active.
    #[cfg(feature = "c-bench")]
    {
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

        // 64-bit index variant (divsufsort64): the same sources built with
        // BUILD_DIVSUFSORT64, which renames every symbol with a `64` suffix.
        cc::Build::new()
            .files([
                "vendor/libdivsufsort/lib/divsufsort.c",
                "vendor/libdivsufsort/lib/sssort.c",
                "vendor/libdivsufsort/lib/trsort.c",
                "vendor/libdivsufsort/lib/utils.c",
            ])
            .include("vendor/include")
            .include("vendor/libdivsufsort/include")
            .opt_level(3)
            .define("HAVE_CONFIG_H", "1")
            .define("BUILD_DIVSUFSORT64", "1")
            // The vendored sufcheck prints saidx_t with a 32-bit format string; it is unused here.
            .warnings(false)
            .define("PROJECT_VERSION_FULL", "\"2.0.2\"")
            .compile("divsufsort64_c");

        println!("cargo:rerun-if-changed=vendor/include");
        println!("cargo:rerun-if-changed=vendor/libdivsufsort");
    }
}
