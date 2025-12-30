mod smachine;
use smachine::vm;
use std::env;
use std::ffi::OsStr;
use std::path::Path;

fn get_stem(file_path: &str) -> Option<&str> {
    Path::new(file_path).file_stem().and_then(OsStr::to_str)
}

fn get_extension(file_path: &str) -> Option<&str> {
    Path::new(file_path).extension().and_then(OsStr::to_str)
}
fn startup() {
    let arguments = env::args();
    let mut debug_flag = false;
    let mut file_path: String = String::from("");
    for arg in arguments {
        match arg.as_str() {
            "--debug" => {
                debug_flag = true;
            }
            _ => {
                file_path = arg.clone();
            }
        }
    }
    if let Some(stem) = get_extension(file_path.as_str()) {
        match stem {
            "bin" => {
                if let Ok(bytecode) = smachine::compiler::read_bin(file_path) {
                    let mut vm = vm::VM::new(bytecode);
                    if debug_flag {
                        println!("Debug mode");
                        vm.debug_run(debug_flag);
                    } else {
                        vm.run();
                    }
                }
            }
            _ => {
                let bin = smachine::compiler::compile_file(&file_path);
                if let Some(bytecode) = bin
                    && let Some(stem) = get_stem(&file_path)
                {
                    let new_path = stem.to_owned() + ".bin";
                    let res = smachine::compiler::write_bin(&new_path, bytecode.clone());
                    if let Ok(_) = res {
                        let mut vm = vm::VM::new(bytecode);
                        if debug_flag {
                            println!("Debug mode");
                            vm.debug_run(debug_flag);
                        } else {
                            vm.run();
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    startup();
}
