use crate::SimulateParams;
use confignoble::model::contextualized::Contextualized;
use confignoble::model::SingleValue;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn run_simulate(
    simulate_params: &SimulateParams,
    kernel_path: &Path,
    root_image_path: &Option<PathBuf>,
    config: &Contextualized,
) -> Result<(), String> {
    let binary = determine_binary(config)?
        .ok_or_else(|| "Could not determine the appropriate QEMU binary".to_string())?;
    if !kernel_path.exists() {
        return Err(format!(
            "Supplied kernel_path {} does not exist",
            kernel_path.display()
        ));
    }

    let mut command = Command::new(binary);
    if let Some(root_image_path) = root_image_path {
        if !root_image_path.exists() {
            return Err(format!(
                "Supplied root_image_path {} does not exist",
                kernel_path.display()
            ));
        }
        command
            .arg("-kernel")
            .arg(format!("{}", kernel_path.display()))
            .arg("-initrd")
            .arg(format!("{}", root_image_path.display()));
    } else {
        command
            .arg("-kernel")
            .arg(format!("{}", kernel_path.display()));
    }

    let machine = determine_machine(config)?;
    if let Some(machine) = &machine {
        command.arg("-machine").arg(machine);
    }

    if let Some(cpu) = determine_cpu_with_properties(config) {
        command.arg("-cpu").arg(cpu);
    }

    command.arg("-nographic").arg("-s");

    if let Some(serial_override) = &simulate_params.serial_override {
        command.args(serial_override.split_whitespace());
    } else {
        if let Some("sabrelite") = machine {
            command.arg("-serial").arg("null");
        }
        command.arg("-serial").arg("mon:stdio");
    }
    command.arg("-m").arg("size=1024M");

    if let Some(extra_qemu_args) = &simulate_params.extra_qemu_args {
        command.args(extra_qemu_args.iter());
    }

    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    if simulate_params.build.is_verbose {
        println!("Running qemu: {:?}", &command);
    }
    let output = command
        .output()
        .map_err(|e| format!("failed to run qemu: {:?}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err("Non-success output status for qemu".into())
    }
}

fn determine_machine(config: &Contextualized) -> Result<Option<&'static str>, String> {
    let kernel_platform = config
        .sel4_config
        .get("KernelX86Platform")
        .or_else(|| config.sel4_config.get("KernelARMPlatform"))
        .ok_or_else(|| {
            "KernelARMPlatform or KernelX86Platform missing but required as a sel4 config option"
        })?;
    if let SingleValue::String(kp) = kernel_platform {
        match kp.as_ref() {
            "imx6" | "sabre" | "sabrelite" => Ok(Some("sabrelite")),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn determine_binary(config: &Contextualized) -> Result<Option<&'static str>, String> {
    let kernel_arch = config.sel4_config.get("KernelArch").ok_or_else(|| {
        "KernelArch is a required config property for simulation to work".to_string()
    })?;
    match kernel_arch {
        SingleValue::String(arch) => match arch.as_ref() {
            "x86" | "x86_64" => Ok(Some("qemu-system-x86_64")),
            "arm" | "aarch32" => Ok(Some("qemu-system-arm")),
            _ => Ok(None),
        },
        _ => Err("Unexpected non-string property value type for KernelArch".to_string()),
    }
}

fn determine_cpu_with_properties(config: &Contextualized) -> Option<String> {
    fn determine_cpu(config: &Contextualized) -> Option<&'static str> {
        if let Some(SingleValue::String(micro)) = config.sel4_config.get("KernelX86MicroArch") {
            match micro.as_ref() {
                "nehalem" | "Nehalem" => Some("Nehalem"),
                _ => None,
            }
        } else {
            None
        }
    }

    if let Some(cpu) = determine_cpu(config) {
        let mut v = vec![cpu.to_string()];
        v.push(toggle_flag_by_property_presence(config, "KernelVTX", "vme"));
        v.push(toggle_flag_by_property_presence(
            config,
            "KernelHugePage",
            "pdpe1gb",
        ));
        v.push(toggle_flag_by_property_presence(
            config,
            "KernelFPUXSave",
            "xsave",
        ));
        v.push(toggle_flag_by_property_presence(
            config,
            "KernelXSaveXSaveOpt",
            "xsaveopt",
        ));
        v.push(toggle_flag_by_property_presence(
            config,
            "KernelXSaveXSaveC",
            "xsavec",
        ));
        v.push(toggle_flag_by_property_presence(
            config,
            "KernelFSGSBaseInst",
            "fsgsbase",
        ));
        v.push(toggle_flag_by_property_presence(
            config,
            "KernelSupportPCID",
            "invpcid",
        ));
        Some(v.join(","))
    } else {
        None
    }
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
