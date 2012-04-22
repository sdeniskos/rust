#[no_core];

enum port<T> {
    blub
}

fn chan<T>(po: port<T>) -> port<T> {
    fail
}

fn prepare_loop() -> port<fn@(int)> {
    let op_port = blub;
    chan(op_port)
}

fn prepare_loop2() -> port<fn@(int)> {
    let op_port = blub;
    chan(op_port)
}

fn main() {
}
