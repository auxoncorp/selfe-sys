/* Copyright (c) 2015 The Robigalia Project Developers
 * Licensed under the Apache License, Version 2.0
 * <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT
 * license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
 * at your option. All files in the project carrying such
 * notice may not be copied, modified, or distributed except
 * according to those terms.
 */

#[macro_use] extern crate maplit;

use std::process::Command;
use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let arch_to_target = hashmap! {
        "x86" => "i686-unknown-linux-gnu",
    };
    for (arch, llvmtriple) in &arch_to_target {
        assert!(Command::new("/usr/bin/env")
            .arg("clang")
            .arg(&*format!("{}.s", arch))
            .args(&["-c", "-target", llvmtriple, "-o", &*format!("{}/{}.o", out_dir, arch)])
            .status().unwrap().success());
        assert!(Command::new("/usr/bin/env")
            .arg("ar")
            .arg("crus")
            .arg(format!("{}/lib{}.a", out_dir,arch))
            .arg(&*format!("{}/{}.o", out_dir, arch))
            .status().unwrap().success());
    }

    if env::var("TARGET").unwrap() == "i686-sel4-unknown" {
        println!("cargo:rustc-link-lib=static=x86");
        println!("cargo:rustc-link-search=native={}", out_dir);
    }

}
