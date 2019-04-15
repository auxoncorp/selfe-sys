use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

extern crate toml;

extern crate bindgen;
use bindgen::Builder;

extern crate confignoble;
use confignoble::build_helpers::*;
use confignoble::compilation::{
    build_sel4, resolve_sel4_source, ResolvedSeL4Source, SeL4BuildMode, SeL4BuildOutcome,
};

extern crate proc_macro2;
use proc_macro2::{Ident, Span, TokenStream};

extern crate quote;
use quote::quote;

extern crate itertools;
use itertools::Itertools;

const BLACKLIST_TYPES: &'static [&'static str] = &[
    "seL4_CPtr",
    "seL4_Word",
    "seL4_Int8",
    "seL4_Int16",
    "seL4_Int32",
    "seL4_Int64",
    "seL4_Uint8",
    "seL4_Uint16",
    "seL4_Uint32",
    "seL4_Uint64",
];

const BUILD_INCLUDE_DIRS: &'static [&'static str] = &[
    "libsel4/include",
    "libsel4/autoconf",
    "kernel/gen_config",
    "libsel4/gen_config",
    "libsel4/arch_include/$ARCH$",
    "libsel4/sel4_arch_include/$SEL4_ARCH$",
];

const KERNEL_INCLUDE_DIRS: &'static [&'static str] = &[
    "libsel4/include",
    "libsel4/arch_include/$ARCH$",
    "libsel4/sel4_arch_include/$SEL4_ARCH$",
    "libsel4/mode_include/$PTR_WIDTH$",
];

fn expand_include_dir(d: &str, arch: &str, sel4_arch: &str, ptr_width: usize) -> String {
    d.replace("$ARCH$", arch)
        .replace("$SEL4_ARCH$", sel4_arch)
        .replace("$PTR_WIDTH$", &format!("{}", ptr_width))
}

fn rustfmt(p: &Path) {
    Command::new("rustfmt")
        .arg(p)
        .output()
        .expect("Failed to rustfmt generated code");
}

fn gen_bindings(
    out_dir: &Path,
    kernel_path: &Path,
    libsel4_build_path: &Path,
    arch: &str,
    sel4_arch: &str,
    ptr_width: usize,
) {
    println!("cargo:rerun-if-file-changed=src/bindgen_wrapper.h");

    let mut bindings = Builder::default()
        .header("src/bindgen_wrapper.h")
        .use_core()
        .ctypes_prefix("ctypes");

    for t in BLACKLIST_TYPES {
        bindings = bindings.blacklist_type(t);
    }

    for d in BUILD_INCLUDE_DIRS {
        bindings = bindings.clang_arg(format!(
            "-I{}",
            libsel4_build_path
                .join(expand_include_dir(d, arch, sel4_arch, ptr_width))
                .display()
        ));
    }

    for d in KERNEL_INCLUDE_DIRS {
        bindings = bindings.clang_arg(format!(
            "-I{}",
            kernel_path
                .join(expand_include_dir(d, arch, sel4_arch, ptr_width))
                .display()
        ));
    }

    let bindings = bindings.generate().expect("bindgen didn't work");

    bindings
        .write_to_file(PathBuf::from(out_dir).join("bindings.rs"))
        .expect("couldn't write bindings");
}

// TODO arm_hyp
fn rust_arch_to_sel4_arch(arch: &str) -> String {
    match arch {
        "arm" => "arm".to_owned(),
        "armv7" => "arm".to_owned(),
        "aarch32" => "arm".to_owned(),
        "aarch64" => "arm".to_owned(),
        "i386" => "x86".to_owned(),
        "i586" => "x86".to_owned(),
        "i686" => "x86".to_owned(),
        "x86_64" => "x86".to_owned(),
        _ => panic!("Unknown arch"),
    }
}

fn rust_arch_to_arch(arch: &str) -> String {
    match arch {
        "arm" => "aarch32".to_owned(),
        "armv7" => "aarch32".to_owned(),
        "aarch32" => "aarch32".to_owned(),
        "aarch64" => "aarch64".to_owned(),
        "i386" => "ia32".to_owned(),
        "i586" => "ia32".to_owned(),
        "i686" => "ia32".to_owned(),
        "x86_64" => "x86_64".to_owned(),
        _ => panic!("Unknown arch"),
    }
}

#[derive(Debug)]
struct BitfieldType {
    name: String,
    is_fault: bool,
    fields: Vec<BitfieldField>,
}

#[derive(Debug, Clone)]
struct BitfieldField {
    name: String,
    width: i64,
}

fn load_bitfields_toml() -> Vec<BitfieldType> {
    println!("cargo:rerun-if-file-changed=codegen/bitfields.toml");
    let bitfields_toml_str = include_str!("codegen/bitfields.toml");
    let bitfields_toml: toml::value::Value =
        toml::from_str(bitfields_toml_str).expect("Parsing bitfields.toml");

    let top_toml = bitfields_toml
        .as_table()
        .expect("Top level of bitfields.toml should be a table");
    let bitfield_types_toml = top_toml
        .get("bitfield_types")
        .and_then(|v| v.as_array())
        .expect("bitfields.toml should have bitfield_types array at the top level");

    let mut types = vec![];
    for raw_type_toml in bitfield_types_toml {
        let type_toml = raw_type_toml
            .as_table()
            .expect("Each bitfield type should be a table");
        let bitfield_type = BitfieldType {
            name: type_toml
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned())
                .expect("name"),
            is_fault: type_toml
                .get("is_fault")
                .and_then(|v| v.as_bool())
                .expect("is_fault"),
            fields: type_toml
                .get("fields")
                .and_then(|v| v.as_array())
                .expect("fields")
                .iter()
                .map(|val| {
                    let t = val.as_table().expect("field");
                    BitfieldField {
                        name: t
                            .get("name")
                            .expect("name")
                            .as_str()
                            .expect("field name must be string")
                            .to_owned(),
                        width: t
                            .get("width")
                            .expect("width")
                            .as_integer()
                            .expect("field width must be integer"),
                    }
                })
                .collect(),
        };

        types.push(bitfield_type);
    }

    types
}

// Aux bitfield types for use with the quote macro
#[derive(Clone)]
struct FieldAccess {
    name: Ident,
    getter: Ident,
    setter: Ident,
    field: BitfieldField
}

fn gen_for_field(f: &BitfieldField) -> TokenStream {
    if f.width == 64 {
        quote! {
            any::<u64>()
        }
    } else {
        let max: u64 = 1 << (f.width - 1);
        quote! {
            0..#max
        }
    }
}

fn gen_bitfield_test(bf: &BitfieldType) -> TokenStream {
    let name = bf.name.clone();
    let is_fault = bf.is_fault;
    // let fields = bf.fields.iter().map(|field_name| FieldView {
    //     name: field_name.clone(),
    //     type_name: bf.name.clone(),
    // });

    let field_names = bf
        .fields
        .iter()
        .map(|f| Ident::new(&f.name.to_owned(), Span::call_site()))
        .collect::<Vec<_>>();

    let param_struct_name = Ident::new(&format!("{}Params", name), Span::call_site());
    let param_struct_fields = field_names.clone();
    let param_struct_code = quote! {
        #[derive(Debug, Clone)]
        struct #param_struct_name {
            #(#param_struct_fields: u64),*
        }
    };

    let constructor = Ident::new(
        &format!("seL4_{}{}_new", if is_fault { "Fault_" } else { "" }, name),
        Span::call_site(),
    );
    let constructor_params = field_names.clone();
    let record_type = if is_fault {
        Ident::new("seL4_Fault", Span::call_site())
    } else {
        Ident::new(&format!("seL4_{}_t", name), Span::call_site())
    };
    let constructor_code = quote! {
        impl #param_struct_name {
            fn create(&self) -> #record_type {
                unsafe {
                    #constructor(
                        #(self.#constructor_params),*
                    )
                }
            }
        }
    };

    // Tuples only work in proptest up to 12 elements. To work around this, set up
    // the generators to have sub-generator tuples in groups of 10.
    let gen_params_fn = Ident::new(&format!("gen_{}_params", name), Span::call_site());
    let fields_gen_code_in_tens = bf.fields.iter().map(gen_for_field).chunks(10);
    let fields_gen_tuples_code = fields_gen_code_in_tens
        .into_iter()
        .map(|chunk| quote! {(#(#chunk),*)});
    let field_names_1 = field_names.clone();
    let field_names_in_tens = field_names.chunks(10);
    let field_name_tuples_code = field_names_in_tens
        .into_iter()
        .map(|chunk| quote! {(#(#chunk),*)});
    let gen_params_fn_code = if field_names.len() > 0 {
        quote! {
            #[allow(unused_parens)]
            fn #gen_params_fn() -> impl Strategy<Value = #param_struct_name> {
                (#(#fields_gen_tuples_code),*)
                    .prop_map(
                        |(#(#field_name_tuples_code),*)|
                        #param_struct_name {
                            #(#field_names_1),*
                        })
            }
        }
    } else {
        quote! {
            fn #gen_params_fn() -> impl Strategy<Value = #param_struct_name> {
                Just(#param_struct_name {})
            }
        }
    };
    let gen_fn = Ident::new(&format!("gen_{}", name), Span::call_site());
    let gen_fn_code = quote! {
        fn #gen_fn() -> impl Strategy<Value = #record_type> {
            #gen_params_fn().prop_map(|params| params.create())
        }
    };

    let field_access = bf.fields
        .iter()
        .map(|f| FieldAccess {
            name: Ident::new(&f.name.to_owned(), Span::call_site()),
            field: f.clone(),
            getter: Ident::new(
                &format!(
                    "seL4_{}{}_ptr_get_{}",
                    if is_fault { "Fault_" } else { "" },
                    name,
                    f.name,
                ),
                Span::call_site(),
            ),
            setter: Ident::new(
                &format!(
                    "seL4_{}{}_ptr_set_{}",
                    if is_fault { "Fault_" } else { "" },
                    name,
                    f.name
                ),
                Span::call_site(),
            ),
        })
        .collect::<Vec<_>>();

    let test_constructor_assertions = field_access.iter().map(|f| {
        let field_name = f.name.clone();
        let field_name_str = format!("{}", field_name);
        let field_getter = f.getter.clone();

        quote! {
            assert_eq!(#field_getter(&mut val), params.#field_name, #field_name_str);
        }
    });
    let test_constructor_code = quote! {
        proptest! {
            #[test]
            #[allow(unused_variables, unused_mut, unused_unsafe, unused_parens)]
            fn constructor_fields(params in #gen_params_fn()) {
                unsafe {
                    let mut val = params.create();
                    #(#test_constructor_assertions)*
                }
            }
        }
    };

    let test_fault_type_code = if bf.is_fault {
        let expected_fault_type = Ident::new(&format!("seL4_Fault_tag_seL4_Fault_{}", bf.name), Span::call_site());

        quote! {
            proptest! {
                #[test]
                #[allow(unused_parens)]
                fn get_fault_type(mut record in #gen_fn()) {
                    unsafe {
                        assert_eq!(seL4_Fault_ptr_get_seL4_FaultType(&mut record), #expected_fault_type as u64);
                    }
                }
            }
        }
    } else {
        quote! { }
    };

    let test_get_set_code = field_access.iter().map(|f| {
        let test_name = Ident::new(&format!("field_{}", f.name), Span::call_site());
        let getter = &f.getter;
        let setter = &f.setter;
        let gen_code = gen_for_field(&f.field);

        quote! {
            proptest! {
                #[test]
                #[allow(unused_parens)]
                fn #test_name(mut record in #gen_fn(), val in #gen_code) {
                    unsafe {
                        #setter(&mut record, val);
                        assert_eq!(#getter(&mut record), val);
                    }
                }
            }
        }
    });

    let mod_name = Ident::new(&format!("{}Test", name), Span::call_site());
    quote::quote! {
        #[cfg(test)]
        mod #mod_name {
            use super::*;
            use proptest::prelude::*;

            #param_struct_code
            #constructor_code
            #gen_params_fn_code
            #gen_fn_code

            #test_constructor_code
            #test_fault_type_code
            #(#test_get_set_code)*
        }
    }
}

fn gen_tests(out_dir: &Path) {
    let bitfield_types = load_bitfields_toml();
    let test_mods_code = bitfield_types.iter().map(gen_bitfield_test);
    let top_level_code = quote! {
        #(#test_mods_code)*
    };

    let out_file = out_dir.join("generated_tests.rs");
    fs::write(&out_file, top_level_code.to_string()).expect("Write generated_tests.rs");
    rustfmt(&out_file);
}

fn main() {
    BuildEnv::request_reruns();
    let BuildEnv {
        cargo_cfg_target_arch,
        cargo_cfg_target_pointer_width,
        out_dir,
        ..
    } = BuildEnv::from_env_vars();
    println!("cargo:rerun-if-file-changed=build.rs");
    println!("cargo:rerun-if-file-changed=src/lib.rs");
    println!("cargo:rerun-if-env-changed=RUSTFLAGS");

    gen_tests(&out_dir);

    let config = load_config_from_env_or_default();
    config.print_boolean_feature_flags();
    let sel4_arch = rust_arch_to_sel4_arch(&cargo_cfg_target_arch);
    let arch = rust_arch_to_arch(&cargo_cfg_target_arch);

    let ResolvedSeL4Source {
        kernel_dir,
        tools_dir,
        util_libs_dir,
    } = resolve_sel4_source(&config.sel4_source, &out_dir.join("sel4_source"))
        .expect("resolve sel4 source");

    let build_dir = if let SeL4BuildOutcome::StaticLib { build_dir } = build_sel4(
        &out_dir,
        &kernel_dir,
        &tools_dir,
        &util_libs_dir,
        &config,
        SeL4BuildMode::Lib,
    ) {
        build_dir
    } else {
        panic!("build_sel4 built us something other than a static library");
    };

    println!("cargo:rustc-link-lib=static=sel4");
    println!(
        "cargo:rustc-link-search=native={}/libsel4",
        build_dir.display()
    );

    gen_bindings(
        &out_dir,
        &kernel_dir,
        &build_dir,
        &sel4_arch,
        &arch,
        cargo_cfg_target_pointer_width,
    );
}
