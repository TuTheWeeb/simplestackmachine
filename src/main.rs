mod smachine;
use smachine::compiler;
use smachine::vm;

fn main() {
    let values = compiler::byte_code_compiler("push8 49 push8 10 sub8 push8 11 add8 prt8");
    if let Some(byts) = values {
        let mut vm = vm::VM::new(byts);
        vm.run();
    }
}
