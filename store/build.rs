fn main() {
    if std::env::var("CARGO_FEATURE_CPP").is_ok() {
        cxx_build::bridge("src/cxxengine.rs")
            .std("c++17")
            .compile("trictrac-cxx");

        println!("cargo:rerun-if-changed=src/cxxengine.rs");
    }
}
