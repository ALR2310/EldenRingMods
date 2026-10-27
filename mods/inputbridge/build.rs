fn main() {
    winresource::WindowsResource::new()
        .compile()
        .expect("embedding the version resource failed (needs rc.exe from the Windows SDK)");
}
