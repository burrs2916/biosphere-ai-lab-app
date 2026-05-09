fn main() {
    if std::env::var("LIBTORCH_USE_PYTORCH").is_ok() || std::env::var("LIBTORCH").is_ok() {
        if let Ok(torch_lib_path) = find_torch_lib_path() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", torch_lib_path);
        }
    }
    tauri_build::build()
}

fn find_torch_lib_path() -> Result<String, String> {
    let python = if std::env::var_os("VIRTUAL_ENV").is_some() {
        "python"
    } else {
        "python3"
    };

    let output = std::process::Command::new(python)
        .args(["-c", "import torch, os; print(os.path.dirname(torch.__file__) + '/lib')"])
        .output()
        .map_err(|e| format!("Failed to run {python}: {}", e))?;

    if !output.status.success() {
        return Err(format!("{python} failed to find torch"));
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if std::path::Path::new(&path).exists() {
        Ok(path)
    } else {
        Err(format!("Torch lib path does not exist: {}", path))
    }
}
