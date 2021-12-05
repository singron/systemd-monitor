extern crate dbus_codegen;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let config = [
        ("systemd-manager.xml", "systemd_manager.rs"),
        ("systemd-service.xml", "systemd_service.rs"),
    ];
    for (xml_path, rs_path) in &config {
        let out_path = std::path::Path::new(&out_dir).join(rs_path);
        let xml = std::fs::read_to_string(xml_path).unwrap();
        let opts = dbus_codegen::GenOpts {
            methodtype: None,
            ..Default::default()
        };
        let output = dbus_codegen::generate(&xml, &opts).unwrap();
        std::fs::write(out_path, output).unwrap();
        println!("cargo:rerun-if-changed={}", xml_path);
    }
}
