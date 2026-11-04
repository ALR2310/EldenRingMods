fn main() {
    // The default is "<Mod>.pdb"; a neutral name keeps the mod's name out of the debug directory.
    println!("cargo:rustc-link-arg-cdylib=/PDBALTPATH:mod.pdb");
    winresource::WindowsResource::new()
        .compile()
        .expect("embedding the version resource failed (needs rc.exe from the Windows SDK)");
}
