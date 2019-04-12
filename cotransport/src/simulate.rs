use crate::BuildParams;
use confignoble::model::contextualized::Contextualized;
use confignoble::model::SingleValue;
use std::path::{PathBuf, Path};
use std::process::{Command, Stdio};

pub fn run_simulate(
    build_params: &BuildParams,
    kernel_path: &Path,
    root_image_path: &Option<PathBuf>,
    config: &Contextualized,
) -> Result<(), String> {
    let binary = determine_binary(config)?.ok_or_else(|| "Could not determine the appropriate QEMU binary".to_string())?;

    let mut command = Command::new(binary);
    command.arg("-kernel")
        .arg(format!("{}", kernel_path.display()));
    if let Some(root_image_path) = root_image_path {
        command.arg("-initrd")
            .arg(format!("{}", root_image_path.display()));
    }
    command.arg("-nographic")
        // if-sabre add:  '-s serial null'?
        .arg("-serial")
        .arg("mon:stdio")
        .arg("-m")
        .arg("size=1024M")
        .arg("-cpu")
        .arg(compute_cpu_properties(config)?);

    command.stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if build_params.is_verbose {
        println!("Running qemu: {:?}", &command);
    }
    let output = command.output().map_err(|e|format!("failed to run qemu: {:?}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err("Non-success output status for qemu".into())
    }
}

fn determine_binary(config: &Contextualized) -> Result<Option<&'static str>, String> {
    let kernel_arch = config.sel4_config.get("KernelArch").ok_or_else(|| "KernelArch is a required config property for simulation to work".to_string())?;
    match kernel_arch {
        SingleValue::String(arch) => {
            match arch.as_ref() {
                "x86" | "x86_64" => Ok(Some("qemu-system-x86_64")),
                "arm" | "aarch32" => Ok(Some("qemu-system-arm")),
                _ => Ok(None)
            }
        },
        _ => Err("Unexpected non-string property value type for KernelArch".to_string())
    }
}

fn compute_cpu_properties(config: &Contextualized) -> Result<String, String> {
    let mut v = Vec::new();
    if let Some(SingleValue::String(micro)) = config.sel4_config.get("KernelX86MicroArch") {
        match micro.as_ref() {
            "nehalem" | "Nehalem" => {
                v.push("Nehalem".to_string());
            },
            _ => ()
        }
    }
    v.push(toggle_flag_by_property_presence(config, "KernelVTX", "vme"));
    v.push(toggle_flag_by_property_presence(config, "KernelHugePage", "pdpe1gb"));
    v.push(toggle_flag_by_property_presence(config, "KernelFPUXSave", "xsave"));
    v.push(toggle_flag_by_property_presence(config, "KernelXSaveXSaveOpt", "xsaveopt"));
    v.push(toggle_flag_by_property_presence(config, "KernelXSaveXSaveC", "xsavec"));
    v.push(toggle_flag_by_property_presence(config, "KernelFSGSBaseInst", "fsgsbase"));
    v.push(toggle_flag_by_property_presence(config, "KernelSupportPCID", "invpcid"));
    Ok(v.join(","))
}

fn toggle_flag_by_property_presence(config: &Contextualized, property: &str, flag: &str) -> String {
    if let Some(val) = config.sel4_config.get(property) {
       match val {
           SingleValue::Boolean(true) => format!("+{}", flag),
           SingleValue::Boolean(false) => format!("-{}", flag),
           _ => format!("+{}", flag),
       }
    } else {
        format!("-{}", flag)
    }
}
