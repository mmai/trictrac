fn main() {
    cxx_build::bridge("src/cxxengine.rs")
        .std("c++17")
        .compile("trictrac-cxx");

    println!("cargo:rerun-if-changed=src/cxxengine.rs");
}
