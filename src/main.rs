mod smachine;
use smachine::compiler;
use smachine::vm;

fn main() {
    let path = "./src/main.s";
    let values = compiler::compile_file(path);
    if let Some(byts) = values {
        let _ = compiler::write_bin("src/main.bin", byts.clone());

        let mut vm = vm::VM::new(byts);
        vm.run();
    }
}
